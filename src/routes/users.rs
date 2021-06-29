use actix_web::{get, post, web, HttpResponse};
use log::info;
use serde_json::json;

use crate::errors::ApiError;
use crate::model::configuration::ApplicationConfiguration;
use crate::model::user::NewUser;
use crate::services::auth::AuthedUser;
use crate::services::users::UserService;

#[post("/users")]
async fn new_user(
    new_user: web::Json<NewUser>,
    user_service: web::Data<UserService>,
    auth: Option<AuthedUser>, //Not needed for now
    configuration: web::Data<ApplicationConfiguration>,
) -> Result<HttpResponse, ApiError> {
    if configuration.allow_account_creation.unwrap_or(false) || auth.is_some() {
        info!("Recording new user {:?}", new_user);
        let data = new_user.into_inner();
        let user =
            web::block(move || user_service.create_user(&data.username, &data.password)).await?;

        Ok(HttpResponse::Created().json(json!({"id": user.id})))
    } else {
        log::debug!("User creation attempt while it's disabled");
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[get("/users")]
async fn list_users(
    user_service: web::Data<UserService>,
    _auth: AuthedUser, //No needed for now
) -> Result<HttpResponse, ApiError> {
    info!("Get all users");

    let users = web::block(move || user_service.list_users()).await?;
    Ok(HttpResponse::Ok().json(users))
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(new_user);
    cfg.service(list_users);
}
