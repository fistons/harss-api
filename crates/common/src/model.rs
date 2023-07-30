use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Error associated to a channel
#[derive(Debug, Serialize)]
pub struct ChannelError {
    pub id: i32,
    pub channel_id: i32,
    pub channel_name: String,
    pub error_timestamp: Option<DateTime<Utc>>,
    pub error_reason: Option<String>,
}

/// A channel with a user's metadata
#[derive(Debug, Serialize)]
pub struct UsersChannel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub last_update: Option<DateTime<Utc>>,
    pub registration_timestamp: DateTime<Utc>,
    pub items_count: Option<i64>,
    pub items_read: Option<i64>,
    pub failure_count: i32,
    pub disabled: bool,
}

/// A HaRss user
#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    #[serde(skip)] // Never ever serialize this field
    pub password: String,
    pub role: UserRole,
}

#[derive(sqlx::Type, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

/// A channel with a user's metadata
#[derive(Debug, Serialize)]
pub struct Channel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub last_update: Option<DateTime<Utc>>,
    pub registration_timestamp: DateTime<Utc>,
    pub failure_count: i32,
    pub disabled: bool,
}

/// Page of elements
#[derive(Debug, Serialize)]
pub struct PagedResult<T> {
    /// Actual content.
    pub content: Vec<T>,
    /// Number of the page.
    pub page_number: u64,
    /// Desired size of the page.
    pub page_size: u64,
    /// Total number of pages.
    pub total_pages: u64,
    /// Number of elements returned.
    pub elements_number: usize,
    /// Total number of elements.
    pub total_items: u64,
}

/// RSS Item representation, with user related data
#[derive(Debug, FromRow, Serialize)]
pub struct UserItem {
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

/// RSS Item representation to be inserted in the database
#[derive(Debug, Serialize)]
pub struct NewItem {
    pub guid: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub content: Option<String>,
    pub fetch_timestamp: DateTime<Utc>,
    pub publish_timestamp: Option<DateTime<Utc>>,
    pub channel_id: i32,
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
