use std::thread;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::errors::ApiError;
use crate::services::auth::AuthedUser;
use crate::services::users::UserService;
use crate::services::GlobalService;
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    login: String,
    password: String,
}

#[post("/refresh")]
pub async fn refresh(
    auth: AuthedUser,
    global_service: web::Data<GlobalService>,
) -> Result<HttpResponse, ApiError> {
    debug!("Refreshing with {}", auth.login);
    thread::spawn(move || global_service.refresh(auth.id));

    Ok(HttpResponse::new(StatusCode::ACCEPTED))
}

#[post("/login")]
pub async fn login(
    login: web::Json<LoginRequest>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, ApiError> {
    let token = crate::services::auth::get_jwt(&login.login, &login.password, user_service)?;
    let mut tok = HashMap::new();

    //TODO: refresh token one day maybe (cf https://git.pedr0.net/twitch/rss-aggregator/-/issues/6)
    tok.insert("access_token", token);
    Ok(HttpResponse::Ok().json(tok))
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(refresh);
    cfg.service(login);
}
