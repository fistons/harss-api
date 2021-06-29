use std::thread;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse};
use log::debug;

use crate::errors::ApiError;
use crate::services::auth::AuthedUser;
use crate::services::GlobalService;

#[post("/refresh")]
pub async fn refresh(
    auth: AuthedUser,
    global_service: web::Data<GlobalService>,
) -> Result<HttpResponse, ApiError> {
    debug!("Refreshing with {}", auth.login);
    thread::spawn(move || global_service.refresh(auth.id));

    Ok(HttpResponse::new(StatusCode::ACCEPTED))
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(refresh);
}
