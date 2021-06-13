use std::str::FromStr;
use std::{error, fmt};

use actix_web::error::BlockingError;
use actix_web::http::{StatusCode, Uri};
use actix_web::{HttpResponse, ResponseError};
use diesel::result::Error as DieselError;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;
use std::collections::HashMap;

/// # Contains the list of all problem types.
mod problems_uri {
    pub const GENERIC: &str = "/problem/generic";
    pub const AUTHENTICATION: &str = "/problem/authentication";
    pub const DATABASE: &str = "/problem/authentication";
}

#[derive(Debug)]
pub struct ApiError {
    problem_type: String,
    title: String,
    detail: String,
    status: StatusCode,
    more: HashMap<String, String>,
}

impl ApiError {
    pub fn unexpected<T>(message: T) -> ApiError
    where
        T: Into<String>,
    {
        ApiError {
            problem_type: problems_uri::GENERIC.into(),
            title: "Unexpected error".into(),
            detail: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            more: HashMap::with_capacity(0),
        }
    }

    pub fn unauthorized<T>(message: T) -> ApiError
    where
        T: Into<String>,
    {
        ApiError {
            problem_type: problems_uri::AUTHENTICATION.into(),
            title: "Authentication error".into(),
            detail: message.into(),
            status: StatusCode::UNAUTHORIZED,
            more: HashMap::with_capacity(0),
        }
    }

    pub fn custom<T>(
        problem_type: Uri,
        title: T,
        detail: T,
        status: StatusCode,
        more: HashMap<String, String>,
    ) -> ApiError
    where
        T: Into<String>,
    {
        ApiError {
            problem_type: problem_type.to_string(),
            title: title.into(),
            detail: detail.into(),
            status,
            more,
        }
    }
}

impl error::Error for ApiError {}

impl Serialize for ApiError {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ApiError", 4 + self.more.len())?;
        state.serialize_field("type", &self.problem_type)?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("status", &self.status.as_u16())?;
        state.serialize_field("detail", &self.detail)?;
        //TODO: find a way to serialize the `more` field
        state.end()
    }
}

impl<E> From<BlockingError<E>> for ApiError
where
    E: fmt::Debug,
{
    fn from(err: BlockingError<E>) -> ApiError {
        log::error!("Blocking error: {:?}", err);
        ApiError::unexpected("Blocked!")
    }
}

impl From<r2d2::Error> for ApiError {
    fn from(err: r2d2::Error) -> Self {
        log::error!("r2d2 error: {}", err);
        ApiError::custom(
            Uri::from_str(problems_uri::DATABASE).unwrap(),
            "Database issue".into(),
            format!("Database issue: {:?}", err),
            StatusCode::INTERNAL_SERVER_ERROR,
            HashMap::with_capacity(0),
        )
    }
}

impl From<DieselError> for ApiError {
    fn from(err: DieselError) -> ApiError {
        log::error!("diesel error: {}", err);
        ApiError::custom(
            Uri::from_str(problems_uri::DATABASE).unwrap(),
            "Database issue".into(),
            format!("Database issue: {:?}", err),
            StatusCode::INTERNAL_SERVER_ERROR,
            HashMap::with_capacity(0),
        )
    }
}

impl From<jwt::Error> for ApiError {
    fn from(err: jwt::Error) -> Self {
        log::error!("jwt error: {}", err);
        ApiError::unauthorized(format!("JWT error: {}", err.to_string()))
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.detail.as_str())
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(json!(self))
    }
}
