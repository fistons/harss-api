pub mod auth;
pub mod model;
pub mod rate_limiting;
pub mod routes;
pub mod services;
pub mod startup;

pub mod errors {
    use actix_web::http::StatusCode;
    use actix_web::{HttpResponse, ResponseError};

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
}
