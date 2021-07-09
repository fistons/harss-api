use actix_web::{post, web, HttpResponse};
use redis::Commands;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::services::users::UserService;
use crate::RefreshTokenStore;

#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    login: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
pub struct RefreshRequest {
    token: String,
}

#[post("/auth/login")]
pub async fn login(
    login: web::Json<LoginRequest>,
    user_service: web::Data<UserService>,
    refresh_token_store: web::Data<RefreshTokenStore>,
) -> Result<HttpResponse, ApiError> {
    let access_token = crate::services::auth::get_jwt_from_login_request(
        &login.login,
        &login.password,
        user_service,
    )?;
    let refresh_token = format!("user.{}.{}", &login.login, Uuid::new_v4().to_string());

    let mut redis = refresh_token_store.store.lock().unwrap();
    let _: () = redis.set_ex(&refresh_token, 1, 60 * 60 * 24 * 5).unwrap();

    Ok(HttpResponse::Ok()
        .json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

#[post("/auth/refresh")]
pub async fn refresh_auth(
    refresh_token: web::Json<RefreshRequest>,
    user_service: web::Data<UserService>,
    refresh_token_store: web::Data<RefreshTokenStore>,
) -> Result<HttpResponse, ApiError> {
    let mut redis = refresh_token_store.store.lock().unwrap();

    if redis.exists(&refresh_token.token).unwrap_or(false) {
        let user_login =
            crate::services::auth::extract_login_from_refresh_token(&refresh_token.token);

        let user = user_service.get_user(&user_login)?;

        /* Create a new JWT */
        let access_token = crate::services::auth::get_jwt(&user)?;

        Ok(HttpResponse::Ok().json(json!({ "access_token": access_token })))
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(login);
    cfg.service(refresh_auth);
}
