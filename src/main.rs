#[macro_use]
extern crate diesel;

use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;

use crate::errors::ApiError;
use crate::model::channel::NewChannel;
use log::{debug, info};
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use std::thread;

mod errors;
mod model;
mod schema;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[get("/channel/{id}")]
async fn get_channel(id: web::Path<i32>, db: web::Data<DbPool>) -> Result<HttpResponse, ApiError> {
    let channel =
        web::block(move || model::channel::db::select_by_id(id.into_inner(), &db.into_inner()))
            .await?;

    Ok(HttpResponse::Ok().json(channel))
}

#[get("/channels")]
async fn get_channels(db: web::Data<DbPool>) -> Result<HttpResponse, ApiError> {
    let channels = web::block(move || model::channel::db::select_all(&db.into_inner())).await?;
    Ok(HttpResponse::Ok().json(channels))
}

#[get("/channel/{chan_id}/items")]
async fn get_items(
    chan_id: web::Path<i32>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, ApiError> {
    let items = web::block(move || {
        model::items::db::get_items_of_channel(chan_id.into_inner(), &pool.into_inner())
    })
    .await?;

    Ok(HttpResponse::Ok().json(items))
}

#[post("/channels")]
async fn new_channel(
    new_channel: web::Json<NewChannel>,
    db: web::Data<DbPool>,
) -> Result<HttpResponse, ApiError> {
    info!("Recording new channel {:?}", new_channel);

    let data = new_channel.into_inner();

    web::block(move || model::channel::db::insert(data, db.into_inner())).await?;

    Ok(HttpResponse::Created().finish())
}

#[post("/channel/{channel_id}/refresh")]
async fn refresh_channel(
    id: web::Path<i32>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let pool = pool.into_inner();
    debug!("Refreshing channel {}", id);

    thread::spawn(move || model::refresh_chan(&pool, id));

    Ok(HttpResponse::Accepted().finish())
}

#[post("/refresh")]
async fn refresh(db: web::Data<DbPool>) -> Result<HttpResponse, ApiError> {
    debug!("Refreshing");

    thread::spawn(move || model::refresh(&db.into_inner()));

    Ok(HttpResponse::new(StatusCode::ACCEPTED))
}

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
            .service(get_channels)
            .service(get_channel)
            .service(new_channel)
            .service(refresh)
            .service(refresh_channel)
            .service(get_items)
            .service(fs::Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(std::env::var("LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")))?
    .run()
    .await
}
