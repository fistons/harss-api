use std::{fmt, error};

use actix_web::error::BlockingError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use diesel::result::Error as DieselError;
use r2d2::Error;
use serde_json::json;
use std::fmt::Error as FmtError;

// TODO: This is a huge mess, fix this 
#[derive(Debug)]
pub struct ApiError {
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

impl<E> From<BlockingError<E>> for ApiError
where
    E: fmt::Debug,
{
    fn from(_: BlockingError<E>) -> ApiError {
        ApiError::new(String::from("Blocked!"))
    }
}

impl From<FmtError> for ApiError {
    fn from(_: FmtError) -> Self {
        ApiError::new(String::from("Error!"))
    }
}

impl From<Error> for ApiError {
    fn from(_: Error) -> Self {
        ApiError::new(String::from("R2D2 Error!"))
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
            .json(json!({ "message": self.message }))
    }
}

impl error::Error for ApiError {}