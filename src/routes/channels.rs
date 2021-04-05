use actix_web::{get, post, web, HttpResponse};
use log::{debug, info};

use crate::errors::ApiError;
use crate::model::channel::NewChannel;
use crate::{model, DbPool};
use std::thread;

#[get("/channel/{id}")]
pub async fn get_channel(
    id: web::Path<i32>,
    db: web::Data<DbPool>,
) -> Result<HttpResponse, ApiError> {
    let channel =
        web::block(move || model::channel::db::select_by_id(id.into_inner(), &db.into_inner()))
            .await?;

    Ok(HttpResponse::Ok().json(channel))
}

#[get("/channels")]
pub async fn get_channels(db: web::Data<DbPool>) -> Result<HttpResponse, ApiError> {
    let channels = web::block(move || model::channel::db::select_all(&db.into_inner())).await?;
    Ok(HttpResponse::Ok().json(channels))
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

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(get_channels)
        .service(get_channel)
        .service(new_channel)
        .service(refresh_channel)
        .service(get_items);
}
