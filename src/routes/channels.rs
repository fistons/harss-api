use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse};
use serde_json::json;

use crate::errors::ApiError;
use crate::model::HttpNewChannel;
use crate::services::auth::AuthenticatedUser;
use crate::services::channels::ChannelService;
use crate::services::items::ItemService;

#[get("/channel/{id}")]
pub async fn get_channel(
    id: web::Path<i32>,
    channel_service: web::Data<ChannelService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let res = channel_service
        .select_by_id_and_user_id(user.id, id.into_inner())
        .await?;

    log::debug!("{:?}", res);

    match res {
        Some(data) => Ok(HttpResponse::Ok().json(data)),
        None => Ok(HttpResponse::new(StatusCode::NOT_FOUND)),
    }
}

#[get("/channels")]
pub async fn get_channels(
    channel_service: web::Data<ChannelService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channels = channel_service.select_all_by_user_id(user.id).await?;
    Ok(HttpResponse::Ok().json(channels))
}

#[post("/channels")]
async fn new_channel(
    new_channel: web::Json<HttpNewChannel>,
    channel_service: web::Data<ChannelService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    log::info!("Recording new channel {:?}", new_channel);

    let data = new_channel.into_inner();
    let channel = channel_service
        .create_or_link_channel(data, user.id)
        .await?;

    Ok(HttpResponse::Created().json(json!({"id": channel.id})))
}

#[get("/channel/{chan_id}/items")]
async fn get_items(
    chan_id: web::Path<i32>,
    items_service: web::Data<ItemService>,
    channel_service: web::Data<ChannelService>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    log::info!("{:?} {}", chan_id, auth.id);

    let chan = channel_service
        .select_by_id_and_user_id(auth.id, chan_id.into_inner())
        .await?;

    if chan == None {
        return Ok(HttpResponse::NotFound().finish());
    }

    let items = items_service.get_items_of_channel(chan.unwrap().id).await?;
    Ok(HttpResponse::Ok().json(items))
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(get_channel)
        .service(get_channels)
        .service(new_channel)
        .service(get_items);
}
