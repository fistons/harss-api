use actix_web::{get, post, web, HttpResponse};
use log::info;
use serde_json::json;

use crate::errors::ApiError;
use crate::model::user::NewUser;
use crate::DbPool;

#[post("/users")]
async fn new_user(
    new_user: web::Json<NewUser>,
    db: web::Data<DbPool>,
) -> Result<HttpResponse, ApiError> {
    info!("Recording new user {:?}", new_user);

    let data = new_user.into_inner();
    let user =
        web::block(move || crate::services::users::create_user(data, &db.into_inner())).await?;

    Ok(HttpResponse::Created().json(json!({"id": user.id})))
}

#[get("/users")]
async fn list_users(db: web::Data<DbPool>) -> Result<HttpResponse, ApiError> {
    info!("Get all users");

    let users = web::block(move || crate::services::users::list_users(&db.into_inner())).await?;
    Ok(HttpResponse::Ok().json(users))
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(new_user);
    cfg.service(list_users);
}
