use std::time::Duration;

use actix_web::{post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::services::users::UserService;
use crate::Cache;

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
    cache: web::Data<Cache>,
) -> Result<HttpResponse, ApiError> {
    let access_token = crate::services::auth::get_jwt_from_login_request(
        &login.login,
        &login.password,
        user_service,
    )?;
    let refresh_token = format!("user.{}.{}", &login.login, Uuid::new_v4().to_string());

    let mut cache = cache.cache.lock().unwrap();
    cache.insert(
        String::from(&refresh_token),
        String::from(&login.login),
        get_duration(),
    );

    Ok(HttpResponse::Ok()
        .json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

#[post("/auth/refresh")]
pub async fn refresh_auth(
    refresh_token: web::Json<RefreshRequest>,
    user_service: web::Data<UserService>,
    cache: web::Data<Cache>,
) -> Result<HttpResponse, ApiError> {
    let mut cache = cache.cache.lock().unwrap();
    match cache.get(&refresh_token.token) {
        Some(user_login) => {
            let user = user_service.get_user(&user_login)?;

            /* Create a new pair of tokens */
            let access_token = crate::services::auth::get_jwt(&user)?;
            let new_refresh_token =
                format!("user.{}.{}", &user.username, Uuid::new_v4().to_string());

            cache.remove(&refresh_token.token); /* Remove the old token */
            cache.insert(
                String::from(&new_refresh_token),
                String::from(&user.username),
                get_duration(),
            );
            Ok(HttpResponse::Ok()
                .json(json!({"access_token": access_token, "refresh_token": new_refresh_token})))
        }
        _ => Ok(HttpResponse::Unauthorized().finish()),
    }
}

fn get_duration() -> Duration {
    Duration::from_secs(60 * 60 * 24 * 5) // 5 Days
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(login);
    cfg.service(refresh_auth);
}
