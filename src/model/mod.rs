use std::fmt::Debug;

use chrono::{DateTime, Utc};
use feed_rs::model::Entry;
use sea_orm::{FromQueryResult, NotSet, Set};
use secrecy::Secret;
use serde::{Deserialize, Serialize, Serializer};

use entity::items;
use entity::sea_orm_active_enums::UserRole;

pub mod opml;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpNewChannel {
    pub name: String,
    pub url: String,
}

/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, FromQueryResult)]
pub struct HttpChannel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub last_update: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, FromQueryResult)]
pub struct HttpUserChannel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub last_update: Option<DateTime<Utc>>,
    pub registration_timestamp: DateTime<Utc>,
    pub items_count: i64,
    #[serde(serialize_with = "ser_with")]
    pub items_read: Option<i64>,
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
}

pub fn item_from_rss_entry(entry: Entry, channel_id: i32) -> items::ActiveModel {
    let title = entry.title.map(|x| x.content);
    let guid = Some(entry.id);
    let url = entry.links.get(0).map(|x| String::from(&x.href[..]));
    let content = entry.summary.map(|x| x.content);

    items::ActiveModel {
        id: NotSet,
        guid: Set(guid),
        title: Set(title),
        url: Set(url),
        content: Set(content),
        fetch_timestamp: Set(Utc::now().into()),
        publish_timestamp: Set(entry.published.map(|x| x.into())),
        channel_id: Set(channel_id),
    }
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
    page: Option<usize>,
    /// * The `size` field should be between 1 and 20
    size: Option<usize>,
}

impl PageParameters {
    /// Return the given page, or 1 if not provided.
    /// If the given page is 0, return 1.
    pub fn get_page(&self) -> usize {
        self.page.unwrap_or(1).max(1)
    }

    /// Return the given size of the page, or 20 if not provided.
    /// The result is clamped between 1 and 200.
    pub fn get_size(&self) -> usize {
        self.size.unwrap_or(20).clamp(1, 200)
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
    pub page: usize,
    /// Desired size of the page.
    pub page_size: usize,
    /// Total number of pages.
    pub total_pages: usize,
    /// Number of elements returned.
    pub elements_number: usize,
    /// Total number of elements.
    pub total_items: usize,
}
