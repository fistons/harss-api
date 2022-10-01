use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use serde_json::json;

use crate::services::AuthenticationError;

pub mod auth;
pub mod channels;
pub mod items;
pub mod users;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Object not found")]
    NotFound(String, i32),
    #[error("Authentication error {0:?}")]
    AuthenticationError(#[from] AuthenticationError),
    #[error("Redis Error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Redis pool Error: {0}")]
    RedisPoolError(#[from] deadpool_redis::PoolError),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::AuthenticationError(error) =>  error.error_response(),
            ApiError::NotFound(object_type, id) => HttpResponse::build(StatusCode::NOT_FOUND)
                .json(json!({"type":"/problem/not-found",
                    "title": "Object not found",
                    "status": 404,
                    "detail": format!("Object of type {} with id {} was not found", object_type, id)})),
            ApiError::DatabaseError(_) | ApiError::RedisError(_) | ApiError::RedisPoolError(_) => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .json(json!({"type":"/problem/database",
                    "title": "Error with the database",
                    "status": 500,
                    "detail": "Unexpected error with the database"})),
            _ => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish(),
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure)
        .configure(channels::configure)
        .configure(items::configure)
        .configure(users::configure);
}
