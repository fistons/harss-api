use actix_web::{get, post, web, HttpResponse};
use log::info;
use serde_json::json;

use crate::errors::ApiError;
use crate::model::configuration::ApplicationConfiguration;
use crate::model::NewUser;
use crate::services::auth::AuthedUser;
use crate::services::users::UserService;

#[post("/users")]
async fn new_user(
    new_user: web::Json<NewUser>,
    user_service: web::Data<UserService>,
    auth: Option<AuthedUser>,
    configuration: web::Data<ApplicationConfiguration>,
) -> Result<HttpResponse, ApiError> {
    if configuration.allow_account_creation.unwrap_or(false)
        || auth.map(|x| x.is_admin()).unwrap_or(false)
    {
        info!("Recording new user {:?}", new_user);
        let data = new_user.into_inner();
        let user = web::block(move || {
            user_service.create_user(&data.username, &data.password, &data.role)
        })
        .await?;

        Ok(HttpResponse::Created().json(json!({"id": user.id})))
    } else {
        log::debug!("User creation attempt while it's disabled or creator is not admin");
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[get("/users")]
async fn list_users(
    user_service: web::Data<UserService>,
    auth: AuthedUser,
) -> Result<HttpResponse, ApiError> {
    if auth.is_admin() {
        info!("Get all users");
        let users = web::block(move || user_service.list_users()).await?;
        Ok(HttpResponse::Ok().json(users))
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(new_user);
    cfg.service(list_users);
}
