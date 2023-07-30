use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use common::model::{NewChannel, PageParameters};
use common::channels::ChannelService;
use common::items::ItemService;
use common::rss;

use crate::auth::AuthenticatedUser;
use crate::routes::ApiError;
use crate::startup::AppState;

#[get("/channel/{id}")]
#[tracing::instrument(skip(app_state))]
pub async fn get_channel(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let res =
        ChannelService::select_by_id_and_user_id(connection, id.into_inner(), user.id).await?;

    match res {
        Some(data) => Ok(HttpResponse::Ok().json(data)),
        None => Ok(HttpResponse::new(StatusCode::NOT_FOUND)),
    }
}

#[get("/channel/{id}/errors")]
#[tracing::instrument(skip(app_state))]
pub async fn get_errors_of_channel(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    if !user.is_admin() {
        return Ok(HttpResponse::Forbidden().finish());
    }

    let errors = ChannelService::select_errors_by_chan_id(connection, id.into_inner()).await?;

    Ok(HttpResponse::Ok().json(errors))
}

#[post("/channel/{id}/read")]
#[tracing::instrument(skip(app_state))]
pub async fn mark_channel_as_read(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    ChannelService::mark_channel_as_read(connection, id.into_inner(), user.id).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[get("/channels")]
#[tracing::instrument(skip(app_state))]
pub async fn get_channels(
    app_state: web::Data<AppState>,
    page: web::Query<PageParameters>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    let channels = ChannelService::select_page_by_user_id(
        connection,
        user.id,
        page.get_page(),
        page.get_size(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(channels))
}

#[post("/channels")]
#[tracing::instrument(skip(app_state))]
async fn new_channel(
    new_channel: web::Json<HttpNewChannel>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let data = new_channel.into_inner();

    let channel = ChannelService::create_or_link_channel(connection, data, user.id).await?;

    Ok(HttpResponse::Created().json(json!({"id": channel.id})))
}

#[get("/channel/{chan_id}/items")]
#[tracing::instrument(skip(app_state))]
async fn get_items_of_channel(
    chan_id: web::Path<i32>,
    page: web::Query<PageParameters>,
    app_state: web::Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    let items = ItemService::get_items_of_channel(
        connection,
        chan_id.into_inner(),
        auth.id,
        page.get_page(),
        page.get_size(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(items))
}

#[post("/channel/{id}/enable")]
#[tracing::instrument(skip(app_state))]
async fn enable_channel(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    if user.is_admin() {
        ChannelService::enable_channel(connection, id.into_inner()).await?;
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
        .service(get_errors_of_channel);
}
