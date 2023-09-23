use actix_web::{post, web, HttpResponse};
use anyhow::{anyhow, Context};
use redis::AsyncCommands;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::common::users::get_user_by_username;

use crate::routes::errors::ApiError;
use crate::startup::AppState;

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
#[tracing::instrument(skip(app_state))]
pub async fn login(
    login: web::Json<LoginRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis_pool = &app_state.redis;

    let access_token =
        crate::auth::get_jwt_from_login_request(&login.login, &login.password, connection).await?;
    let refresh_token = format!("user.{}.{}", &login.login, Uuid::new_v4());

    let mut redis = redis_pool.get().await?;
    redis
        .set_ex::<_, _, ()>(&refresh_token, 1, 60 * 60 * 24 * 5)
        .await?;

    Ok(HttpResponse::Ok()
        .json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

#[post("/auth/refresh")]
#[tracing::instrument(skip(app_state))]
pub async fn refresh_auth(
    refresh_token: web::Json<RefreshRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis_pool = &app_state.redis;
    let mut redis = redis_pool.get().await?;
    let token = refresh_token.token.expose_secret();
    let token_exists = redis.exists::<_, bool>(token).await?;

    if token_exists {
        let user_login = crate::auth::extract_login_from_refresh_token(token);
        let user = get_user_by_username(connection, user_login)
            .await
            .context("Could not get user")?
            .ok_or_else(|| anyhow!("Unknown user"))?;
        /* Create a new JWT */
        let access_token = crate::auth::get_jwt(&user).await?;

        Ok(HttpResponse::Ok().json(json!({ "access_token": access_token })))
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
    cfg.service(refresh_auth);
}
