#[macro_use]
extern crate diesel;

use actix_web::{App, get, HttpResponse, HttpServer, post, Responder, web};
use actix_web::http::StatusCode;
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;

use crate::model::Channel;

mod schema;
mod model;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[get("/channel/{id}")]
async fn get_channel(id: web::Path<i32>, db: web::Data<DbPool>) -> web::Json<Channel> {
    let connection = db.get().unwrap();
    web::Json(web::block(move || model::db::select_by_id(id.into_inner(), &connection)).await.unwrap())
}

#[get("/channels")]
async fn get_channels(db: web::Data<DbPool>) -> web::Json<Vec<Channel>> {
    let connection = db.get().unwrap();
    web::Json(web::block(move || model::db::select_all(&connection)).await.unwrap())
}


#[post("/channels")]
async fn new_channel(new_channel: web::Json<model::NewChannel>, db: web::Data<DbPool>) -> HttpResponse {
    println!("Recording new channel {:?}", new_channel);

    let connection = db.get().unwrap();
    let data = new_channel.into_inner();

    web::block(move || model::db::insert(data, &connection)).await.unwrap();

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
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
