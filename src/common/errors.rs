use feed_rs::parser::ParseFeedError;

#[derive(thiserror::Error, Debug)]
pub enum RssParsingError {
    #[error("Non OK Http status returned: {0}")]
    NonOkStatus(u16),
    #[error("Error while reading response: {0}")]
    ReadResponsError(#[from] reqwest::Error),
    #[error("Error while building request: {0}")]
    HttpError(#[from] reqwest_middleware::Error),
    #[error("Parse error: {0}")]
    ParseFeedError(#[from] ParseFeedError),
}

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("Rss parsing error: {0}")]
    RssError(#[from] RssParsingError),
    #[error("Given passwords doesn't match")]
    NonMatchingPassword,
    #[error(transparent)]
    FeedValidationError(#[from] anyhow::Error),
}
