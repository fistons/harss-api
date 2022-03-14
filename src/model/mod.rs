use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use entity::sea_orm_active_enums::UserRole;
use entity::users::Model;

pub mod configuration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpNewChannel {
    pub name: String,
    pub url: String,
}

/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpChannel {
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpNewUser {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpUser {
    pub id: i32,
    pub username: String,
    pub role: UserRole,
}

impl From<entity::users::Model> for HttpUser {
    fn from(u: Model) -> Self {
        HttpUser {
            id: u.id,
            username: u.username,
            role: u.role,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpNewItem {
    pub guid: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub content: Option<String>,
    pub read: bool,
    pub channel_id: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpItem {
    pub id: i32,
    pub guid: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub content: Option<String>,
    pub read: bool,
    #[serde(skip_serializing)]
    pub channel_id: i32,
}

impl HttpNewItem {
    /// Create an item to be inserted in the database, from a rss item.
    pub fn from_rss_item(item: rss::Item, channel_id: i32) -> HttpNewItem {
        let title = item.title;
        let guid = item.guid.map(|x| x.value);
        let url = item.link;
        let content = item.description;
        let read = false;

        HttpNewItem {
            guid,
            title,
            url,
            content,
            read,
            channel_id,
        }
    }
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
