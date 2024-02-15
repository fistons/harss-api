use actix_web::{get, post, put, web, HttpResponse};

use crate::common::items::*;

use crate::auth::AuthenticatedUser;
use crate::model::{IdListParameter, ItemNotesRequest, PageParameters, ReadStarredParameters};
use crate::routes::errors::ApiError;
use crate::startup::AppState;

#[get("/items")]
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
pub async fn unread_item(
    ids: web::Json<IdListParameter>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    set_item_read(connection, user.id, ids.into_inner().ids, false).await?;

    Ok(HttpResponse::Accepted().finish())
}

#[put("/item/{item_id}/notes")]
pub async fn add_item_notes(
    item_id: web::Path<i32>,
    request: web::Json<ItemNotesRequest>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    add_notes(
        connection,
        request.into_inner().notes,
        user.id,
        item_id.into_inner(),
    )
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[get("/item/{id}")]
pub async fn get_item(
    id: web::Path<i32>,
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    let item = get_one_item(connection, id.into_inner(), user.id).await?;

    match item {
        Some(item) => Ok(HttpResponse::Ok().json(item)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_all_items)
        .service(star_items)
        .service(unstar_items)
        .service(read_item)
        .service(unread_item)
        .service(add_item_notes)
        .service(get_item);
}
