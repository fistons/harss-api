use actix_web::{get, post, web, HttpResponse};

use common::items::*;

use crate::auth::AuthenticatedUser;
use crate::model::{IdListParameter, PageParameters, ReadStarredParameters};
use crate::routes::errors::ApiError;
use crate::startup::AppState;

#[get("/items")]
#[tracing::instrument(skip(app_state))]
pub async fn get_all_items(
    page: web::Query<PageParameters>,
    read_starred: web::Query<ReadStarredParameters>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    let items = get_items_of_user(
        connection,
        None,
        read_starred.starred,
        read_starred.read,
        user.id,
        page.get_page(),
        page.get_size(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(items))
}

#[post("/items/star")]
#[tracing::instrument(skip(app_state))]
pub async fn star_items(
    ids: web::Json<IdListParameter>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    set_item_starred(connection, user.id, ids.into_inner().ids, true).await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/items/unstar")]
#[tracing::instrument(skip(app_state))]
pub async fn unstar_items(
    ids: web::Json<IdListParameter>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    set_item_starred(connection, user.id, ids.into_inner().ids, false).await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/items/read")]
#[tracing::instrument(skip(app_state))]
pub async fn read_item(
    ids: web::Json<IdListParameter>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    set_item_read(connection, user.id, ids.into_inner().ids, true).await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/items/unread")]
#[tracing::instrument(skip(app_state))]
pub async fn unread_item(
    ids: web::Json<IdListParameter>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    set_item_read(connection, user.id, ids.into_inner().ids, false).await?;

    Ok(HttpResponse::Accepted().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_all_items)
        .service(star_items)
        .service(unstar_items)
        .service(read_item)
        .service(unread_item);
}
