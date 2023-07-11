use std::fmt::Debug;

use chrono::{DateTime, Utc};
use sea_orm::FromQueryResult;
use secrecy::Secret;
use serde::{Deserialize, Serialize, Serializer};

use entity::sea_orm_active_enums::UserRole;

pub mod opml;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpNewChannel {
    pub name: String,
    pub url: String,
}

/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, FromQueryResult)]
pub struct HttpChannel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub last_update: Option<DateTime<Utc>>,
    pub failure_count: i32,
    pub disabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, FromQueryResult)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, FromQueryResult)]
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

#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct HttpUser {
    pub id: i32,
    pub username: String,
    pub role: UserRole,
}

/// RSS Item representation, with user related data
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
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

/// Filter parameters on read/starred items' status
#[derive(Debug, Deserialize)]
pub struct ReadStarredParameters {
    pub read: Option<bool>,
    pub starred: Option<bool>,
}

/// Represent a list of IDs (could be item, channel, etc)
#[derive(Debug, Deserialize)]
pub struct IdListParameter {
    pub ids: Vec<i32>,
}

/// # Paging parameters
#[derive(Debug, Deserialize)]
pub struct PageParameters {
    /// * The `page` field should be superior or equals to 1
    page: Option<u64>,
    /// * The `size` field should be between 1 and 20
    size: Option<u64>,
}

impl PageParameters {
    /// Return the given page, or 1 if not provided.
    /// If the given page is 0, return 1.
    pub fn get_page(&self) -> u64 {
        self.page.unwrap_or(1).max(1)
    }

    /// Return the given size of the page, or 20 if not provided.
    /// The result is clamped between 1 and 200.
    pub fn get_size(&self) -> u64 {
        self.size.unwrap_or(20u64).clamp(1, 200)
    }
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

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: Secret<String>,
    pub new_password: Secret<String>,
    pub confirm_password: Secret<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOtherPasswordRequest {
    pub new_password: Secret<String>,
    pub confirm_password: Secret<String>,
}
