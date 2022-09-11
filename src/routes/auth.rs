use actix_web::{post, web, HttpResponse};
use anyhow::{anyhow, Context};
use deadpool_redis::{redis::cmd, Pool};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::routes::ApiError;
use crate::startup::ApplicationServices;

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
#[tracing::instrument(skip(services, redis_pool), level = "debug")]
pub async fn login(
    login: web::Json<LoginRequest>,
    services: web::Data<ApplicationServices>,
    redis_pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let access_token = crate::services::auth::get_jwt_from_login_request(
        &login.login,
        login.password.expose_secret(),
        &services.user_service,
    )
    .await?;
    let refresh_token = format!("user.{}.{}", &login.login, Uuid::new_v4());

    let mut redis = redis_pool.get().await?;

    cmd("SETEX")
        .arg(&refresh_token)
        .arg(60 * 60 * 24 * 5) // 5 days
        .arg(1)
        .query_async::<_, ()>(&mut redis)
        .await
        .unwrap();

    Ok(HttpResponse::Ok()
        .json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

#[post("/auth/refresh")]
#[tracing::instrument(skip(redis_pool, services), level = "debug")]
pub async fn refresh_auth(
    refresh_token: web::Json<RefreshRequest>,
    services: web::Data<ApplicationServices>,
    redis_pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut redis = redis_pool.get().await?;

    let token = refresh_token.token.expose_secret();
    let token_exists = cmd("EXISTS")
        .arg(token)
        .query_async::<_, bool>(&mut redis)
        .await
        .unwrap();

    if token_exists {
        let user_login = crate::services::auth::extract_login_from_refresh_token(token);
        let user = services
            .user_service
            .get_user(user_login)
            .await
            .context("Could not get user")?
            .ok_or_else(|| anyhow!("Unknown user"))?;
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
