use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, ResponseError};
use rand::Rng;
use serde_json::json;

use crate::services::{AuthenticationError, ServiceError};

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
    #[error("{0}")]
    ServiceError(#[from] ServiceError),
    #[error("Password mismatch")]
    PasswordMismatch,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

//TODO: Improve error translation, this sucks ass. I should probably remove a layer here
impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::AuthenticationError(error) =>  error.error_response(),
            ApiError::NotFound(object_type, id) => HttpResponse::NotFound()
                .json(json!({"type":"/problem/not-found",
                    "title": "Object not found",
                    "status": 404,
                    "detail": format!("Object of type {} with id {} was not found", object_type, id)})),
            ApiError::DatabaseError(_) | ApiError::RedisError(_) | ApiError::RedisPoolError(_) => HttpResponse::InternalServerError()
                .json(json!({"type":"/problem/database",
                    "title": "Error with the database",
                    "status": 500,
                    "detail": "Unexpected error with the database"})),
            ApiError::PasswordMismatch => HttpResponse::BadRequest().json(json!({"type":"/problem/password-mismatch", "title": "Passwords does not match", "status": 400, "title": "Passwords does not match"})),
            _ => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish(),
        }
    }
}

#[get("/api/v1/ping")]
#[tracing::instrument]
pub async fn ping() -> HttpResponse {
    let mut rng = rand::thread_rng();
    let quotes = [
        "We Will Send Unto Them...Only You",
        "The Slayer Has Entered The Building",
        "The Slayer Has Control Of The BFG",
        "The Demons...They Are Everywhere",
        "No",
        "The Cost Of Progress",
        "May You Rot In Hell",
        "God Rested On The Seventh Day",
        "Rip And Tear",
        "Your Affinity For Guns Is Apparent",
        "The Mark Of The Doom Slayer",
        "Don't Leave That Plasma Cutter Running",
        "Now You Know Why We Do This",
        "Opening The Gate Is Everything",
        "May We Never Need You Again",
    ];

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(quotes[rng.gen_range(0..quotes.len())])
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure)
        .configure(channels::configure)
        .configure(items::configure)
        .configure(users::configure);
}
