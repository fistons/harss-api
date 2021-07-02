use std::thread;

use actix_web::{get, post, web, HttpResponse};
use log::{debug, info};
use serde_json::json;

use crate::errors::ApiError;
use crate::model::channel::NewChannel;
use crate::services::auth::AuthedUser;
use crate::services::channels::ChannelService;
use crate::services::items::ItemService;
use crate::services::GlobalService;

#[get("/channel/{id}")]
pub async fn get_channel(
    id: web::Path<i32>,
    channel_service: web::Data<ChannelService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    let channel =
        web::block(move || channel_service.select_by_id_and_user_id(auth.id, id.into_inner()))
            .await?;

    Ok(HttpResponse::Ok().json(channel))
}

#[get("/channels")]
pub async fn get_channels(
    channel_service: web::Data<ChannelService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    let channels = web::block(move || channel_service.select_all_by_user_id(auth.id)).await?;
    Ok(HttpResponse::Ok().json(channels))
}

#[post("/channels")]
async fn new_channel(
    new_channel: web::Json<NewChannel>,
    channel_service: web::Data<ChannelService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    info!("Recording new channel {:?}", new_channel);

    let mut data = new_channel.into_inner();
    data.set_user_id(auth.id);

    let channel = web::block(move || channel_service.insert(data)).await?;

    Ok(HttpResponse::Created().json(json!({"id": channel.id})))
}

#[post("/channel/{channel_id}/refresh")]
async fn refresh_channel(
    id: web::Path<i32>,
    global_service: web::Data<GlobalService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    debug!("Refreshing channel {}", id);

    thread::spawn(move || global_service.refresh_chan(id, auth.id));

    Ok(HttpResponse::Accepted().finish())
}

#[get("/channel/{chan_id}/items")]
async fn get_items(
    chan_id: web::Path<i32>,
    items_service: web::Data<ItemService>,
    channel_service: web::Data<ChannelService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    let items = web::block(move || {
        let chan = channel_service.select_by_id_and_user_id(chan_id.into_inner(), auth.id)?;
        items_service.get_items_of_channel(chan.id)
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