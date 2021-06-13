use std::fmt::Error as FmtError;
use std::{error, fmt};

use actix_web::error::BlockingError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use diesel::result::Error as DieselError;
use r2d2::Error;
use serde_json::json;

// TODO: This is a huge mess, fix this
#[derive(Debug)]
pub struct ApiError {
    message: String,
    status: StatusCode,
}

impl ApiError {
    pub fn default<T>(message: T) -> ApiError
    where
        T: Into<String>,
    {
        ApiError {
            message: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn unauthorized<T>(message: T) -> ApiError
    where
        T: Into<String>,
    {
        ApiError {
            message: message.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }
}

impl From<DieselError> for ApiError {
    fn from(_: DieselError) -> ApiError {
        ApiError::default("Database is all fucked up, yo")
    }
}

impl<E> From<BlockingError<E>> for ApiError
where
    E: fmt::Debug,
{
    fn from(_: BlockingError<E>) -> ApiError {
        ApiError::default("Blocked!")
    }
}

impl From<FmtError> for ApiError {
    fn from(_: FmtError) -> Self {
        ApiError::default("Error!")
    }
}

impl From<Error> for ApiError {
    fn from(_: Error) -> Self {
        ApiError::default("R2D2 Error!")
    }
}

impl From<jwt::Error> for ApiError {
    fn from(err: jwt::Error) -> Self {
        ApiError::default(format!("JWT error: {}", err.to_string()))
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(json!({ "message": self.message }))
    }
}

impl error::Error for ApiError {}
