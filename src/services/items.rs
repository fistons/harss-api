use std::sync::Arc;

use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*};

use entity::channel_users;
use entity::channels;
use entity::items;
use entity::items::Entity as Item;

use crate::errors::ApiError;
use crate::model::{HttpNewItem, PagedResult};

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

    pub async fn get_items_of_user(
        &self,
        user_id: i32,
        page: usize,
        page_size: usize,
    ) -> Result<PagedResult<items::Model>, ApiError> {
        log::debug!(
            "Getting items of user {} Page {}, Size {}",
            user_id,
            page,
            page_size
        );

        let items_paginator = Item::find()
            .join(JoinType::RightJoin, items::Relation::Channels.def())
            .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
            .filter(channel_users::Column::UserId.eq(user_id))
            .order_by_desc(items::Column::Id)
            .paginate(self.db.as_ref(), page_size);

        let total_pages = items_paginator.num_pages().await?;
        let total_items = items_paginator.num_items().await?;
        let content = items_paginator.fetch_page(page - 1).await?;
        let elements_number = content.len();
        
        Ok(PagedResult {
            content,
            page,
            page_size,
            total_pages,
            elements_number,
            total_items,
        })
    }
}
