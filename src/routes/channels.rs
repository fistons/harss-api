use actix_web::http::StatusCode;
use actix_web::{delete, get, post, web, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use crate::common::rss;

use crate::auth::AuthenticatedUser;
use crate::model::{PageParameters, RegisterChannelRequest};
use crate::routes::errors::ApiError;
use crate::startup::AppState;

#[get("/channel/{id}")]
pub async fn get_channel(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channel_service = &app_state.channel_service;
    let res = channel_service
        .select_by_id_and_user_id(*id, user.id)
        .await?;

    match res {
        Some(data) => Ok(HttpResponse::Ok().json(data)),
        None => Ok(HttpResponse::new(StatusCode::NOT_FOUND)),
    }
}

#[delete("/channel/{id}")]
pub async fn unsubscribe_channel(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channel_service = &app_state.channel_service;
    channel_service.unsubscribe_channel(*id, user.id).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[get("/channel/{id}/errors")]
pub async fn get_errors_of_channel(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channel_service = &app_state.channel_service;

    if !user.is_admin() {
        return Ok(HttpResponse::Forbidden().finish());
    }

    let errors = channel_service
        .select_errors_by_chan_id(*id, user.id)
        .await?;

    Ok(HttpResponse::Ok().json(errors))
}

#[post("/channel/{id}/read")]
pub async fn mark_channel_as_read(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channel_service = &app_state.channel_service;
    channel_service.mark_channel_as_read(*id, user.id).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[get("/channels")]
pub async fn get_channels(
    app_state: web::Data<AppState>,
    page: web::Query<PageParameters>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channel_service = &app_state.channel_service;
    let channels = channel_service
        .select_page_by_user_id(user.id, page.get_page(), page.get_size())
        .await?;
    Ok(HttpResponse::Ok().json(channels))
}

#[post("/channels")]
async fn new_channel(
    new_channel: web::Json<RegisterChannelRequest>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let data = new_channel.into_inner();
    let channel_service = &app_state.channel_service;

    let channel_id = channel_service
        .create_or_link_channel(&data.url, data.name, data.notes, user.id)
        .await?;

    Ok(HttpResponse::Created().json(json!({"id": channel_id})))
}

#[get("/channel/{chan_id}/items")]
async fn get_items_of_channel(
    chan_id: web::Path<i32>,
    page: web::Query<PageParameters>,
    app_state: web::Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let item_service = &app_state.item_service;
    let items = item_service
        .get_items_of_user(
            Some(*chan_id),
            None,
            None,
            auth.id,
            page.get_page(),
            page.get_size(),
        )
        .await?;

    Ok(HttpResponse::Ok().json(items))
}

#[post("/channel/{id}/enable")]
async fn enable_channel(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let channel_service = &app_state.channel_service;

    if user.is_admin() {
        channel_service.enable_channel(*id).await?;
        Ok(HttpResponse::Accepted().finish())
    } else {
        Ok(HttpResponse::Forbidden().finish())
    }
}

#[derive(Deserialize)]
struct QueryParamsUrl {
    url: String,
}

#[get("/channels/search")]
async fn search_channels(
    _user: AuthenticatedUser,
    query_params: web::Query<QueryParamsUrl>,
) -> Result<HttpResponse, ApiError> {
    let found_channels = rss::download_and_look_for_rss(&query_params.url).await?;
    Ok(HttpResponse::Ok().json(found_channels))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(search_channels)
        .service(mark_channel_as_read)
        .service(get_channel)
        .service(get_channels)
        .service(new_channel)
        .service(get_items_of_channel)
        .service(enable_channel)
        .service(get_errors_of_channel)
        .service(unsubscribe_channel);
}
