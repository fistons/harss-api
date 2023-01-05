use std::env::var;

use actix_governor::{GovernorConfig, GovernorConfigBuilder, KeyExtractor};
use actix_web::dev::ServiceRequest;

#[derive(Clone)]
pub struct UserToken;

impl KeyExtractor for UserToken {
    type Key = String;
    type KeyExtractionError = &'static str;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        req.headers()
            .get("Authorization")
            .and_then(|token| token.to_str().ok())
            .map(|token| token.trim().to_owned())
            .ok_or("You must be authenticated")
    }

    fn response_error(&self, err: Self::KeyExtractionError) -> actix_web::Error {
        actix_web::error::ErrorUnauthorized(err.to_string())
    }
}

pub fn build_rate_limiting_conf() -> GovernorConfig<UserToken> {
    GovernorConfigBuilder::default()
        .per_second(
            var("RATE_LIMITING_BUCKET_SIZE")
                .unwrap_or_else(|_| "10".to_owned())
                .parse()
                .unwrap(),
        )
        .burst_size(
            var("RATE_LIMITING_BUCKET_SIZE")
                .unwrap_or_else(|_| "100".to_owned())
                .parse()
                .unwrap(),
        )
        .key_extractor(UserToken)
        .use_headers()
        .finish()
        .unwrap()
}
