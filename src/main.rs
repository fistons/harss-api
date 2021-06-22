#[macro_use]
extern crate diesel;

use crate::services::channels::ChannelService;
use crate::services::items::ItemService;
use crate::services::users::UserService;
use actix_files as fs;
use actix_web::{App, HttpServer};
use diesel::r2d2::ConnectionManager;
use diesel::{sql_types, SqliteConnection};
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use std::sync::Arc;

mod errors;
mod model;
mod routes;
mod schema;
mod services;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
no_arg_sql_function!(last_insert_rowid, sql_types::Integer);

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenv::dotenv().ok();

    // Init Logger
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Always,
    )])
    .unwrap();

    // set up database connection pool
    let connection_spec = std::env::var("DATABASE_URL").unwrap_or_else(|_| String::from("rss.db"));
    let manager = ConnectionManager::<SqliteConnection>::new(connection_spec);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let item_service = ItemService {
        pool: Arc::new(pool.clone()),
    };

    let channel_service = ChannelService {
        pool: Arc::new(pool.clone()),
    };

    let user_service = UserService {
        pool: Arc::new(pool.clone()),
    };

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .data(item_service.clone())
            .data(channel_service.clone())
            .data(user_service.clone())
            .configure(routes::channels::configure)
            .configure(routes::service::configure)
            .configure(routes::users::configure)
            .service(fs::Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(std::env::var("LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")))?
    .run()
    .await
}
