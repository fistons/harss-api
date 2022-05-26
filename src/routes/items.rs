use actix_web::{get, web, HttpResponse};

use crate::errors::ApiError;
use crate::model::PageParameters;
use crate::services::auth::AuthenticatedUser;
use crate::ItemService;

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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_all_items);
}
