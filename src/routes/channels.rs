use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse};
use actix_xml::Xml;
use serde_json::json;

use crate::errors::ApiError;
use crate::model::opml::Opml;
use crate::model::{HttpNewChannel, PageParameters};
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
    page: web::Query<PageParameters>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channels = channel_service
        .select_all_by_user_id(user.id, page.get_page(), page.get_size())
        .await?;
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
    page: web::Query<PageParameters>,
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

    let items = items_service
        .get_items_of_channel(chan.unwrap().id, page.get_page(), page.get_size())
        .await?;
    Ok(HttpResponse::Ok().json(items))
}

#[post("/channels/import")]
async fn import_opml(
    channel_service: web::Data<ChannelService>,
    auth: AuthenticatedUser,
    opml: Xml<Opml>,
) -> Result<HttpResponse, ApiError> {
    let opml = opml.into_inner();
    for channel in opml.body.flatten_outlines() {
        channel_service
            .create_or_link_channel(channel, auth.id)
            .await?;
    }

    Ok(HttpResponse::Created().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_channel)
        .service(get_channels)
        .service(new_channel)
        .service(get_items)
        .service(import_opml);
}
