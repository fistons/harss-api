use actix_web::{post, web, HttpResponse};
use redis::Commands;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::services::users::UserService;
use crate::RefreshTokenStore;

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    login: String,
    password: Secret<String>,
}

#[derive(Deserialize, Debug)]
pub struct RefreshRequest {
    token: Secret<String>,
}

#[post("/auth/login")]
#[tracing::instrument(skip(user_service, refresh_token_store), level = "debug")]
pub async fn login(
    login: web::Json<LoginRequest>,
    user_service: web::Data<UserService>,
    refresh_token_store: web::Data<RefreshTokenStore>,
) -> Result<HttpResponse, ApiError> {
    let access_token = crate::services::auth::get_jwt_from_login_request(
        &login.login,
        login.password.expose_secret(),
        user_service,
    )
    .await?;
    let refresh_token = format!("user.{}.{}", &login.login, Uuid::new_v4());

    let mut redis = refresh_token_store.store.lock().unwrap();
    redis
        .set_ex::<_, _, ()>(&refresh_token, 1, 60 * 60 * 24 * 5)
        .unwrap();

    Ok(HttpResponse::Ok()
        .json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

#[post("/auth/refresh")]
#[tracing::instrument(skip(refresh_token_store, user_service), level = "debug")]
pub async fn refresh_auth(
    refresh_token: web::Json<RefreshRequest>,
    user_service: web::Data<UserService>,
    refresh_token_store: web::Data<RefreshTokenStore>,
) -> Result<HttpResponse, ApiError> {
    let mut redis = refresh_token_store.store.lock().unwrap();

    let token = refresh_token.token.expose_secret();
    if redis.exists(token).unwrap_or(false) {
        let user_login = crate::services::auth::extract_login_from_refresh_token(token);

        let user = user_service
            .get_user(user_login)
            .await?
            .ok_or_else(|| ApiError::not_found("User not found"))?;
        /* Create a new JWT */
        let access_token = crate::services::auth::get_jwt(&user).await?;

        Ok(HttpResponse::Ok().json(json!({ "access_token": access_token })))
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
    cfg.service(refresh_auth);
}
