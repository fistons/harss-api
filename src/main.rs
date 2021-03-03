#[macro_use]
extern crate diesel;

use actix_web::{App, get, HttpResponse, HttpServer, post, Responder, web};
use actix_web::http::StatusCode;
use diesel::r2d2::ConnectionManager;
use diesel::SqliteConnection;

use diesel::prelude::*; 


mod schema;
mod model;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[get("/")]
async fn hello() -> impl Responder {
    "Hi!"
}

#[post("/channel")]
async fn new_channel(new_channel: web::Json<model::NewChannel>, db: web::Data<DbPool>) -> HttpResponse {
    println!("Recording new channel {:?}", new_channel);
    
    let connection = db.get().unwrap();
    let data = new_channel.into_inner();

    web::block(move || {
        use crate::schema::channels::dsl::*;

        
        diesel::insert_into(channels).values(&data).execute(connection).unwrap();
        
        Ok(())
    });

    HttpResponse::new(StatusCode::ACCEPTED)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    // set up database connection pool
    let connspec = "./test.db";
    let manager = ConnectionManager::<SqliteConnection>::new(connspec);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");


    println!("Starting");
    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .service(hello)
            .service(new_channel)
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
