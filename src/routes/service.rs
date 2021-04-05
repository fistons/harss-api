use crate::errors::ApiError;
use crate::{model, DbPool};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse};
use log::debug;
use std::thread;

#[post("/refresh")]
pub async fn refresh(db: web::Data<DbPool>) -> Result<HttpResponse, ApiError> {
    debug!("Refreshing");

    thread::spawn(move || model::refresh(&db.into_inner()));

    Ok(HttpResponse::new(StatusCode::ACCEPTED))
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(refresh);
}
