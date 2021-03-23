use std::fmt;

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use diesel::result::Error as DieselError;
use serde_json::json;

#[derive(Debug)]
struct ApiError {
    message: String,
}

impl ApiError {
    pub fn new(message: String) -> ApiError {
        ApiError { message }
    }
}

impl From<DieselError> for ApiError {
    fn from(_: DieselError) -> ApiError {
        ApiError::new(String::from("Database is all fucked up, yo"))
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).json(json!({ "message": self.message }))
    }
}
