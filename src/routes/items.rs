use actix_rt::spawn;
use actix_web::{get, post, web, HttpResponse};

use crate::errors::ApiError;
use crate::model::PageParameters;
use crate::services::auth::AuthenticatedUser;
use crate::{GlobalService, ItemService};

#[get("/items")]
pub async fn get_all_items(
    page: web::Query<PageParameters>,
    item_service: web::Data<ItemService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    log::debug!("Get all items for {} and {:?}", user.login, page);

    let items = item_service
        .get_items_of_user(user.id, page.get_page(), page.get_size())
        .await?;
    Ok(HttpResponse::Ok().json(items))
}

#[post("/refresh")]
pub async fn refresh_items(
    global_service: web::Data<GlobalService>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    spawn(async move { global_service.refresh_channel_of_user(user.id).await });
    Ok(HttpResponse::Accepted().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_all_items).service(refresh_items);
}
