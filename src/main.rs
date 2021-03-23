#[macro_use]
extern crate diesel;

use actix_web::http::StatusCode;
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;
use rss::Channel as RssChannel;

use crate::model::channel::{Channel, NewChannel};
use crate::model::items::Item;

mod model;
mod schema;
mod errors;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[get("/channel/{id}")]
async fn get_channel(id: web::Path<i32>, db: web::Data<DbPool>) -> web::Json<Channel> {
    let connection = db.get().unwrap();
    web::Json(
        web::block(move || model::channel::db::select_by_id(id.into_inner(), &connection))
            .await
            .unwrap(),
    )
}

#[get("/channels")]
async fn get_channels(db: web::Data<DbPool>) -> web::Json<Vec<Channel>> {
    let connection = db.get().unwrap();
    web::Json(
        web::block(move || model::channel::db::select_all(&connection))
            .await
            .unwrap(),
    )
}

#[get("/items/{chan_id}")]
async fn get_items(chan_id: web::Path<i32>, db: web::Data<DbPool>) -> web::Json<Vec<Item>> {
    let connection = db.get().unwrap();
    web::Json(
        web::block(move || {
            model::items::db::get_items_of_channel(chan_id.into_inner(), &connection)
        })
        .await
        .unwrap(),
    )
}

#[post("/channels")]
async fn new_channel(new_channel: web::Json<NewChannel>, db: web::Data<DbPool>) -> HttpResponse {
    println!("Recording new channel {:?}", new_channel);

    let connection = db.get().unwrap();
    let data = new_channel.into_inner();

    web::block(move || model::channel::db::insert(data, &connection))
        .await
        .unwrap();

    HttpResponse::new(StatusCode::ACCEPTED)
}

#[post("/refresh/{channel_id}")]
async fn refresh_channel(id: web::Path<i32>, db: web::Data<DbPool>) -> HttpResponse {
    let id = id.into_inner();
    let connection = db.get().unwrap();
    println!("Refreshing channel {}", id);

    let channel = web::block(move || model::channel::db::select_by_id(id, &db.get().unwrap()))
        .await
        .unwrap();

    println!("Fetching {}", &channel.name);

    let content = reqwest::get(&channel.url)
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let rss_channel = RssChannel::read_from(&content[..]).unwrap();
    for item in rss_channel.items.into_iter() {
        let i = model::items::NewItem::from_rss_item(item, channel.id);
        model::items::db::insert(i, &connection).unwrap();
    }

    HttpResponse::new(StatusCode::ACCEPTED)
}

#[post("/refresh")]
async fn refresh(db: web::Data<DbPool>) -> HttpResponse {
    println!("Refreshing");
    let connection = db.get().unwrap();

    let channels = web::block(move || model::channel::db::select_all(&db.get().unwrap()))
        .await
        .unwrap();

    for channel in channels {
        println!("Fetching {}", &channel.name);

        let content = reqwest::get(&channel.url)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let rss_channel = RssChannel::read_from(&content[..]).unwrap();
        for item in rss_channel.items.into_iter() {
            let i = model::items::NewItem::from_rss_item(item, channel.id);
            model::items::db::insert(i, &connection).unwrap();
        }
    }

    HttpResponse::new(StatusCode::ACCEPTED)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // set up database connection pool
    let connection_spec = "./test.db";
    let manager = ConnectionManager::<SqliteConnection>::new(connection_spec);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    println!("Starting");
    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .service(get_channels)
            .service(get_channel)
            .service(new_channel)
            .service(refresh)
            .service(refresh_channel)
            .service(get_items)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
