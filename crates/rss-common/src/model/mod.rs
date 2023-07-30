use std::fmt::Debug;

use chrono::{DateTime, Utc};
use secrecy::Secret;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpNewChannel {
    pub name: String,
    pub url: String,
}

/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct HttpChannel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub last_update: Option<DateTime<Utc>>,
    pub failure_count: i32,
    pub disabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct HttpUserChannel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub last_update: Option<DateTime<Utc>>,
    pub registration_timestamp: DateTime<Utc>,
    pub items_count: i64,
    #[serde(serialize_with = "ser_with")]
    pub items_read: Option<i64>,
    pub failure_count: i32,
    pub disabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct HttpChannelError {
    pub id: i32,
    pub channel_id: i32,
    pub channel_name: String,
    pub error_timestamp: Option<DateTime<Utc>>,
    pub error_reason: String,
}

/// Serialize an optional i64, defaulting to 0 if its None
fn ser_with<S: Serializer>(id: &Option<i64>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_i64(id.unwrap_or(0i64))
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpNewUser {
    pub username: String,
    pub password: Secret<String>,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpUser {
    pub id: i32,
    pub username: String,
    pub role: UserRole,
}

/// RSS Item representation, with user related data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpUserItem {
    pub id: i32,
    pub guid: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub content: Option<String>,
    pub fetch_timestamp: DateTime<Utc>,
    pub publish_timestamp: Option<DateTime<Utc>>,
    pub read: bool,
    pub starred: bool,
    pub channel_id: i32,
    pub channel_name: String,
}

/// # Paged result
///
/// Return the elements matching the request in page, alongside the page context.
#[derive(Debug, Serialize)]
pub struct PagedResult<T>
where
    T: Serialize + Debug,
{
    /// Actual content.
    pub content: Vec<T>,
    /// Number of the page.
    pub page: u64,
    /// Desired size of the page.
    pub page_size: u64,
    /// Total number of pages.
    pub total_pages: u64,
    /// Number of elements returned.
    pub elements_number: usize,
    /// Total number of elements.
    pub total_items: u64,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct FoundRssChannel {
    url: String,
    title: String,
}

impl FoundRssChannel {
    pub fn new(url: &str, title: &str) -> Self {
        FoundRssChannel {
            url: url.to_owned(),
            title: title.to_owned(),
        }
    }
}
