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
            role: u.role
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewItem {
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

impl NewItem {
    /// Create an item to be inserted in the database, from a rss item.
    pub fn from_rss_item(item: rss::Item, channel_id: i32) -> NewItem {
        let title = item.title;
        let guid = item.guid.map(|x| x.value);
        let url = item.link;
        let content = item.description;
        let read = false;

        NewItem {
            guid,
            title,
            url,
            content,
            read,
            channel_id,
        }
    }
}
