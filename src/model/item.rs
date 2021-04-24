use serde::{Deserialize, Serialize};

use crate::schema::items;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "items"]
pub struct NewItem {
    pub guid: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub content: Option<String>,
    pub read: bool,
    pub channel_id: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct Item {
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
            title,
            guid,
            url,
            content,
            channel_id,
            read,
        }
    }
}
