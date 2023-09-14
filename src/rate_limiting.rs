use actix_governor::governor::middleware::StateInformationMiddleware;
use actix_governor::{
    GovernorConfig, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError,
};
use actix_web::dev::ServiceRequest;
use std::env::var;

#[derive(Clone)]
pub struct UserToken;

impl KeyExtractor for UserToken {
    type Key = String;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        let auth = req
            .headers()
            .get("Authorization")
            .and_then(|token| token.to_str().ok())
            .map(|token| token.trim().to_owned());

        if let Some(auth) = auth {
            return Ok(auth);
        }

        req.request()
            .peer_addr()
            .map(|x| x.ip().to_string())
            .ok_or_else(|| SimpleKeyExtractionError::new("Can't extract key"))
    }
}

pub fn build_rate_limiting_conf() -> GovernorConfig<UserToken, StateInformationMiddleware> {
    GovernorConfigBuilder::default()
        .per_second(
            var("RATE_LIMITING_FILL_RATE")
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
