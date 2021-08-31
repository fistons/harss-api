use actix_web::{get, post, delete, web, HttpResponse};
use serde_json::json;
use crate::model::NewCategory;
use crate::services::categories::CategoryService;
use crate::services::auth::AuthedUser;
use crate::errors::ApiError;

#[post("/categories")]
async fn new_category(
    new_cat: web::Json<NewCategory>,
    category_service: web::Data<CategoryService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    log::info!("Recording new category {:?}", new_cat);

    let data = new_cat.into_inner();
    let category = web::block(move || category_service.create_category(data, auth.id)).await?;

    Ok(HttpResponse::Created().json(json!({"id": category.id})))
}

#[get("/categories")]
async fn get_categories(
    category_service: web::Data<CategoryService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    
    let categories = web::block(move || category_service.list_categories_of_user(auth.id)).await?;

    Ok(HttpResponse::Ok().json(json!(categories)))
}

#[delete("/category/{category_id}")]
async fn delete_categories(
    category_id: web::Path<i32>,
    category_service: web::Data<CategoryService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {

    web::block(move || category_service.delete_category(category_id.into_inner(), auth.id)).await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(new_category);
    cfg.service(get_categories);
    cfg.service(delete_categories);
}