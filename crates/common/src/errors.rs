use feed_rs::parser::ParseFeedError;

#[derive(thiserror::Error, Debug)]
pub enum RssParsingError {
    #[error("Non OK Http status returned: {0}")]
    NonOkStatus(u16),
    #[error("Error while fetching the feed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    ParseFeedError(#[from] ParseFeedError),
}
