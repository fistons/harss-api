use actix_rt::spawn;
use actix_web::{get, post, web, HttpResponse};

use crate::errors::ApiError;
use crate::model::{IdListParameter, PageParameters, ReadStarredParameters};
use crate::services::auth::AuthenticatedUser;
use crate::services::items::ItemService;
use crate::services::GlobalService;

#[get("/items")]
#[tracing::instrument(skip(item_service), level = "debug")]
pub async fn get_all_items(
    page: web::Query<PageParameters>,
    read_starred: web::Query<ReadStarredParameters>,
    item_service: web::Data<ItemService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let items = item_service
        .get_items_of_user(
            user.id,
            page.get_page(),
            page.get_size(),
            read_starred.read,
            read_starred.starred,
        )
        .await?;
    Ok(HttpResponse::Ok().json(items))
}

#[post("/items/star")]
#[tracing::instrument(skip(item_service), level = "debug")]
pub async fn star_items(
    ids: web::Json<IdListParameter>,
    item_service: web::Data<ItemService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    item_service
        .set_item_starred(user.id, ids.into_inner().ids, true)
        .await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/items/unstar")]
#[tracing::instrument(skip(item_service), level = "debug")]
pub async fn unstar_items(
    ids: web::Json<IdListParameter>,
    item_service: web::Data<ItemService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    item_service
        .set_item_starred(user.id, ids.into_inner().ids, false)
        .await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/item/{item_id}/read")]
#[tracing::instrument(skip(item_service), level = "debug")]
pub async fn read_item(
    ids: web::Json<IdListParameter>,
    item_service: web::Data<ItemService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    item_service
        .set_item_read(user.id, ids.into_inner().ids, true)
        .await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/item/{item_id}/unread")]
#[tracing::instrument(skip(item_service), level = "debug")]
pub async fn unread_item(
    ids: web::Json<IdListParameter>,
    item_service: web::Data<ItemService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    item_service
        .set_item_read(user.id, ids.into_inner().ids, false)
        .await?;

    Ok(HttpResponse::Accepted().finish())
}

#[post("/refresh")]
#[tracing::instrument(skip(global_service), level = "debug")]
pub async fn refresh_items(
    global_service: web::Data<GlobalService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    spawn(async move { global_service.refresh_channel_of_user(user.id).await });
    Ok(HttpResponse::Accepted().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_all_items)
        .service(refresh_items)
        .service(star_items)
        .service(unstar_items)
        .service(read_item)
        .service(unread_item);
}
