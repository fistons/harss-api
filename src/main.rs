#[macro_use]
extern crate diesel;

use actix_files as fs;
use actix_web::{ App, HttpServer};
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;

use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

mod errors;
mod model;
mod schema;

mod routes;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

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

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .configure(routes::channels::configure)
            .configure(routes::service::configure)
            .service(fs::Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(std::env::var("LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")))?
    .run()
    .await
}
