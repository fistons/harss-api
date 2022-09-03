use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::{error, fmt};

use actix_web::http::{StatusCode, Uri};
use actix_web::{HttpResponse, ResponseError};
use deadpool_redis::PoolError;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;

/// # Contains the list of all problem types.
mod problems_uri {
    pub const AUTHENTICATION: &str = "/problem/authentication";
    pub const DATABASE: &str = "/problem/database";
    pub const NOT_FOUND: &str = "/problem/not-found";
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

    pub fn not_found<T>(message: T) -> ApiError
    where
        T: Into<String>,
    {
        ApiError {
            problem_type: problems_uri::NOT_FOUND.into(),
            title: "Object not found".into(),
            detail: message.into(),
            status: StatusCode::NOT_FOUND,
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

impl From<sea_orm::DbErr> for ApiError {
    fn from(err: sea_orm::DbErr) -> Self {
        tracing::error!("sea_orm error: {}", err);
        ApiError::custom(
            Uri::from_str(problems_uri::DATABASE).unwrap(),
            "Database issue".into(),
            format!("Database issue: {:?}", err),
            StatusCode::INTERNAL_SERVER_ERROR,
            HashMap::with_capacity(0),
        )
    }
}

impl From<PoolError> for ApiError {
    fn from(err: PoolError) -> Self {
        tracing::error!("deadpool error: {}", err);
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
        tracing::error!("jwt error: {}", err);
        ApiError::unauthorized(format!("JWT error: {}", err))
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
