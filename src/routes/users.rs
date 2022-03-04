use actix_web::{get, HttpResponse, post, web};
use log::info;
use serde_json::json;

use entity::sea_orm_active_enums::UserRole;

use crate::errors::ApiError;
use crate::model::{HttpNewUser, HttpUser};
use crate::model::configuration::ApplicationConfiguration;
use crate::services::auth::AuthedUser;
use crate::services::users::UserService;

#[post("/users")]
async fn new_user(
    new_user: web::Json<HttpNewUser>,
    user_service: web::Data<UserService>,
    auth: Option<AuthedUser>,
    configuration: web::Data<ApplicationConfiguration>,
) -> Result<HttpResponse, ApiError> {
    let admin = auth.map(|x| x.is_admin()).unwrap_or(false);
    if configuration.allow_account_creation.unwrap_or(false)
        || admin
    {
        info!("Recording new user {:?}", new_user);
        let data = new_user.into_inner();

        if data.role == UserRole::Admin && admin {
            log::debug!("Tried to create a new admin with a non admin user");
            return Ok(HttpResponse::Unauthorized().finish());
        }

        let user = user_service.create_user(&data.username, &data.password, data.role).await?;

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
        let users: Vec<HttpUser> = user_service.list_users().await?.into_iter()
            .map(|u| u.into())
            .collect();
        Ok(HttpResponse::Ok().json(users))
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(new_user)
        .service(list_users);
}
