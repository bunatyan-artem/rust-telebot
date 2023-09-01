use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::payloads::SendMessage;
use teloxide::requests::JsonRequest;
use teloxide::types::KeyboardMarkup;
use teloxide::types::KeyboardButton;
use teloxide::Bot;
use teloxide::requests::Requester;
use sqlx::FromRow;
use sqlx::SqlitePool;

const DB_URL: &str = "sqlite://sqlite.db";

#[derive(Clone, FromRow, Debug)]
struct Word {
    id: i64,
    engl: String,
    rus: String,
}

#[derive(Clone, FromRow, Debug)]
struct User {
    id: i64,
    user_id: i64,
    word_id: i64,
    count: i64,
}

#[derive(Clone, FromRow, Debug)]
struct Time {
    next_time: String,
}

async fn help(bot: Bot, msg: Message) -> JsonRequest<SendMessage> {
    let mut answer = "Это бот для заучивания английских слов. ".to_string();
    answer.push_str("Чтобы начать напишите /learn и появится слово на английском. ");
    answer.push_str("Если вы его знаете, нажмите на /known, не знаете - /notknown, ");
    answer.push_str("не хотите учить - /delete, посмотреть перевод - /translate, ");
    answer.push_str("выйти - /menu. Слова периодически надо будет повторять.") ;
    answer.push_str("Для этого напишите /repeat. Если вы вспомнили слово, нажмите ");
    answer.push_str("/remembered, иначе /notremembered. Кнопка /progress показывает ");
    answer.push_str("количество изучаемых и уже известных слов. /new УДАЛЯЕТ весь прогресс.");

    let keyboard = [
        format!("/learn"),
        format!("/repeat"),
        format!("/progress"),
        format!("/new")
        ].map(|but| KeyboardButton::new(but));

    bot.send_message(msg.chat.id, answer).reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
}

async fn menu(bot: Bot, msg: Message) -> JsonRequest<SendMessage> {
    let keyboard = [
        format!("/learn"),
        format!("/repeat"),
        format!("/progress"),
        format!("/new")
        ].map(|but| KeyboardButton::new(but));

    bot.send_message(msg.chat.id, "выход в главное меню").reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
}

async fn new(bot: Bot, msg: Message) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    sqlx::query("DELETE FROM users WHERE user_id = ?;").bind(msg.chat.id.to_string()).execute(&db).await.unwrap();

    let keyboard = [
        format!("/learn"),
        format!("/repeat"),
        format!("/progress"),
        format!("/new")
        ].map(|but| KeyboardButton::new(but));

    bot.send_message(msg.chat.id, "прогресс удален").reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
}

async fn learn(bot: Bot, msg: Message) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    
    loop {
        let words = sqlx::query_as::<_, Word>("SELECT * FROM words ORDER BY RANDOM() LIMIT 1;").fetch_all(&db).await.unwrap();
        let alr = sqlx::query_as::<_, User>("SELECT id, user_id, word_id, count FROM users WHERE user_id = ? AND word_id = ?;").bind(msg.chat.id.to_string()).bind(words[0].id).fetch_all(&db).await.unwrap();
        
        if alr.len() == 0 { 
            let id = words[0].id;
            let engl = &words[0].engl;

            let keyboard = [
                format!("/known {id}"),
                format!("/notknown {id}"),
                format!("/delete {id}"),
                format!("/translatel {id}"),
                format!("/menu")
                ].map(|but| KeyboardButton::new(but));

            return bot.send_message(msg.chat.id, engl).reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
        }
    }
}

async fn repeat(bot: Bot, msg: Message) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let words = sqlx::query_as::<_, User>("SELECT id, user_id, word_id, count FROM users WHERE user_id = ? AND next_time < ? AND count > -1 AND count < 10 LIMIT 1;").bind(msg.chat.id.to_string()).bind(msg.date.to_string()).fetch_all(&db).await.unwrap();
    
    if words.len() == 0 {
        let next = sqlx::query_as::<_, Time>("SELECT next_time FROM users WHERE user_id = ? AND count > -1 AND count < 10 ORDER BY next_time;").bind(msg.chat.id.to_string()).fetch_all(&db).await.unwrap();
        let answer: String;

        if next.len() == 0 {
            answer = "вы еще не начали учить слова".to_string();
        }
        else {
            let time = &next[0].next_time;
            answer = format!("Сейчас нет слов для повторения\n(появятся {time})");
        }

        let keyboard = [
            format!("/learn"),
            format!("/repeat"),
            format!("/progress"),
            format!("/new")
            ].map(|but| KeyboardButton::new(but));

        return bot.send_message(msg.chat.id, answer).reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
    }

    let word = sqlx::query_as::<_, Word>("SELECT * FROM words WHERE id = ?;").bind(words[0].word_id.to_string()).fetch_all(&db).await.unwrap();    
    let id = word[0].id;
    
    let keyboard = [
        format!("/remembered {id}"),
        format!("/notremembered {id}"),
        format!("/translater {id}"),
        format!("/menu")
        ].map(|but| KeyboardButton::new(but));

    bot.send_message(msg.chat.id, &word[0].engl).reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
}

async fn progress(bot: Bot, msg: Message) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let not = sqlx::query_as::<_, User>("SELECT id, user_id, word_id, count FROM users WHERE user_id = ? AND count = -1;").bind(msg.chat.id.to_string()).fetch_all(&db).await.unwrap();
    let still = sqlx::query_as::<_, User>("SELECT id, user_id, word_id, count FROM users WHERE user_id = ? AND count > -1 AND count < 10;").bind(msg.chat.id.to_string()).fetch_all(&db).await.unwrap();
    let alr = sqlx::query_as::<_, User>("SELECT id, user_id, word_id, count FROM users WHERE user_id = ? AND count = 10;").bind(msg.chat.id.to_string()).fetch_all(&db).await.unwrap();

    let keyboard = [
                format!("/learn"),
                format!("/repeat"),
                format!("/progress"),
                format!("/new")
                ].map(|but| KeyboardButton::new(but));
    
    let mut answer = "вы изучаете - ".to_string();
    answer.push_str(&still.len().to_string());
    answer.push_str("\nуже знаете - ");
    answer.push_str(&alr.len().to_string());
    answer.push_str("\nосталось - ");
    answer.push_str(&(5000 - not.len() - still.len() - alr.len()).to_string());

    bot.send_message(msg.chat.id, answer).reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
}

async fn known(bot: Bot, msg: Message, word_id: i64) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    sqlx::query("INSERT INTO users (user_id, word_id, next_time, count) VALUES (?, ?, ?, 10);").bind(msg.chat.id.to_string()).bind(word_id.to_string()).bind(msg.date.to_string()).execute(&db).await.unwrap();
    
    learn(bot, msg).await
}

async fn not_known(bot: Bot, msg: Message, word_id: i64) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap(); 
    sqlx::query("INSERT INTO users (user_id, word_id, next_time, count) VALUES (?, ?, ?, 0);")
        .bind(msg.chat.id.to_string()).bind(word_id.to_string())
        .bind(msg.date.checked_add_signed(chrono::Duration::seconds(1800)).unwrap().to_string())
        .execute(&db).await.unwrap();

    learn(bot, msg).await
}

async fn remembered(bot: Bot, msg: Message, word_id: i64) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let alr = sqlx::query_as::<_, User>("SELECT id, user_id, word_id, count FROM users WHERE user_id = ? AND word_id = ?;").bind(msg.chat.id.to_string()).bind(word_id.to_string()).fetch_all(&db).await.unwrap();
    
    let interval: i64;
    match alr[0].count {
        0 => interval = 7200,
        1 => interval = 28800,
        2 => interval = 86400,
        3 => interval = 259200,
        4 => interval = 604800,
        5 => interval = 1814400,
        6 => interval = 7884000,
        7 => interval = 15768000,
        8 => interval = 31536000,
        i64::MIN..=-1_i64 | 9_i64..=i64::MAX => interval = 0,
    };

    sqlx::query("UPDATE users SET next_time = ?, count = count + 1 WHERE user_id = ? AND word_id = ?;")
        .bind(msg.date.checked_add_signed(chrono::Duration::seconds(interval)).unwrap().to_string())
        .bind(msg.chat.id.to_string()).bind(word_id.to_string())
        .execute(&db).await.unwrap();

    repeat(bot, msg).await
}

async fn not_remembered(bot: Bot, msg: Message, word_id: i64) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    sqlx::query("UPDATE users SET next_time = ? WHERE user_id = ? AND word_id = ?;")
        .bind(msg.date.checked_add_signed(chrono::Duration::seconds(28800)).unwrap().to_string())
        .bind(msg.chat.id.to_string()).bind(word_id.to_string())
        .execute(&db).await.unwrap();

    repeat(bot, msg).await
}

async fn delete(bot: Bot, msg: Message, word_id: i64) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    sqlx::query("INSERT INTO users (user_id, word_id, next_time, count) VALUES (?, ?, ?, -1);").bind(msg.chat.id.to_string()).bind(word_id.to_string()).bind(msg.date.to_string()).execute(&db).await.unwrap();
    
    learn(bot, msg).await
}

async fn translate_l(bot: Bot, msg: Message, word_id: i64) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let word = sqlx::query_as::<_, Word>("SELECT * FROM words WHERE id = ?;").bind(word_id.to_string()).fetch_all(&db).await.unwrap();
    
    let mut answer = word[0].engl.to_string();
    answer.push_str(" - ");
    answer.push_str(&word[0].rus.to_string());

    let keyboard = [
                format!("/known {word_id}"),
                format!("/notknown {word_id}"),
                format!("/delete {word_id}"),
                format!("/translatel {word_id}"),
                format!("/menu")
                ].map(|but| KeyboardButton::new(but));

    bot.send_message(msg.chat.id, answer).reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
}

async fn translate_r(bot: Bot, msg: Message, word_id: i64) -> JsonRequest<SendMessage> {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let word = sqlx::query_as::<_, Word>("SELECT * FROM words WHERE id = ?;").bind(word_id.to_string()).fetch_all(&db).await.unwrap();
    
    let mut answer = word[0].engl.to_string();
    answer.push_str(" - ");
    answer.push_str(&word[0].rus.to_string());

    let keyboard = [
                format!("/remembered {word_id}"),
                format!("/notremembered {word_id}"),
                format!("/translater {word_id}"),
                format!("/menu")
                ].map(|but| KeyboardButton::new(but));

    bot.send_message(msg.chat.id, answer).reply_markup(KeyboardMarkup::new([keyboard]).one_time_keyboard(true))
}

#[tokio::main]
async fn main() {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let result = sqlx::query_as::<_, User>("SELECT id, user_id, word_id, count FROM users;").fetch_all(&db).await.unwrap();
    println!("{:?}", result);

    pretty_env_logger::init();
    log::info!("Starting Bilbot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "")]
    Help,
    #[command(description = "")]
    Start,
    #[command(description = "главное меню")]
    Menu,
    #[command(description = "удалить прогресс")]
    New,
    #[command(description = "учить новые слова")]
    Learn,
    #[command(description = "повторять заученные слова")]
    Repeat,
    #[command(description = "посмотреть свой прогресс")]
    Progress,
    #[command(description = "слово уже известно")]
    Known(i64),
    #[command(description = "неизвестное слово")]
    NotKnown(i64),
    #[command(description = "слово уже известно (во время повторения)")]
    Remembered(i64),
    #[command(description = "неизвестное слово (во время повторения)")]
    NotRemembered(i64),
    #[command(description = "не учить данное слово")]
    Delete(i64),
    #[command(description = "показать перевод")]
    TranslateL(i64),
    #[command(description = "показать перевод")]
    TranslateR(i64),
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => help(bot, msg).await.await?,
        Command::Start => help(bot, msg).await.await?,
        Command::Menu => menu(bot, msg).await.await?,
        Command::New => new(bot, msg).await.await?,
        Command::Learn => learn(bot, msg).await.await?,
        Command::Repeat => repeat(bot, msg).await.await?,
        Command::Progress => progress(bot, msg).await.await?,
        Command::Known(word_id) => known(bot, msg, word_id).await.await?,
        Command::NotKnown(word_id) => not_known(bot, msg, word_id).await.await?,
        Command::Remembered(word_id) => remembered(bot, msg, word_id).await.await?,
        Command::NotRemembered(word_id) => not_remembered(bot, msg, word_id).await.await?,
        Command::Delete(word_id) => delete(bot, msg, word_id).await.await?,
        Command::TranslateL(word_id) => translate_l(bot, msg, word_id).await.await?,
        Command::TranslateR(word_id) => translate_r(bot, msg, word_id).await.await?,
    };

    Ok(())
}