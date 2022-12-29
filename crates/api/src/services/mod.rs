use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use feed_rs::parser::ParseFeedError;

pub mod auth;
pub mod channels;
pub mod items;
pub mod rss_detector;
pub mod users;

#[derive(thiserror::Error, Debug)]
pub enum RssParsingError {
    #[error("Non OK Http status returned: {0}")]
    NonOkStatus(u16),
    #[error("Error while fetching the feed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    ParseFeedError(#[from] ParseFeedError),
}

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    SqlError(#[from] sea_orm::DbErr),
    #[error("Rss parsing error: {0}")]
    RssError(#[from] RssParsingError),
    #[error(transparent)]
    FeedValidationError(#[from] anyhow::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum AuthenticationError {
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("Invalid JWT")]
    InvalidJwt(#[from] jwt::Error),
    #[error("JWT Token is expired. Please renew it")]
    ExpiredToken,
    #[error("Unsupported authentication scheme, Only Basic HTTP and JWT Bearer are supported")]
    UnknownAuthScheme,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ResponseError for AuthenticationError {
    fn status_code(&self) -> StatusCode {
        match self {
            AuthenticationError::Unauthorized(_)
            | AuthenticationError::InvalidJwt(_)
            | AuthenticationError::ExpiredToken => StatusCode::UNAUTHORIZED,
            AuthenticationError::Forbidden(_) => StatusCode::FORBIDDEN,
            AuthenticationError::UnknownAuthScheme => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        //TODO: Let's have a real response next time
        HttpResponse::build(self.status_code()).finish()
    }
}
