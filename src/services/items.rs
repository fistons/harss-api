use std::sync::Arc;

use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*};

use entity::items;
use entity::items::Entity as Item;

use crate::errors::ApiError;
use crate::model::HttpNewItem;

#[derive(Clone)]
pub struct ItemService {
    db: Arc<DatabaseConnection>,
}

impl ItemService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db: Arc::new(db) }
    }

    pub async fn insert(&self, new_item: HttpNewItem) -> Result<items::Model, ApiError> {
        let item = items::ActiveModel {
            id: NotSet,
            guid: Set(new_item.guid),
            title: Set(new_item.title),
            url: Set(new_item.url),
            content: Set(new_item.content),
            read: Set(false),
            channel_id: Set(new_item.channel_id),
        };

        Ok(item.insert(self.db.as_ref()).await?)
    }

    pub async fn get_items_of_channel(&self, chan_id: i32) -> Result<Vec<items::Model>, ApiError> {
        log::debug!("Getting items of channel {}", chan_id);

        Ok(Item::find()
            .filter(items::Column::ChannelId.eq(chan_id))
            .all(self.db.as_ref())
            .await?)
    }
}
