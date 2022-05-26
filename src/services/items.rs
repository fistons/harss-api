use feed_rs::model::Entry;
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*};

use entity::channel_users;
use entity::channel_users::Entity as ChannelUsers;
use entity::items;
use entity::items::Entity as Item;
use entity::users_items;

use crate::errors::ApiError;
use crate::model::{item_from_rss_entry, PagedResult};

#[derive(Clone)]
pub struct ItemService {
    db: Arc<DatabaseConnection>,
}

impl ItemService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db: Arc::new(db) }
    }

    pub async fn insert(&self, entry: Entry, channel_id: i32) -> Result<items::Model, ApiError> {
        log::trace!("Inserting item {:?}", entry);

        let item = item_from_rss_entry(entry, channel_id);

        let item = item.insert(self.db.as_ref()).await?;

        for channel_user in self.get_users_of_channel(channel_id).await? {
            let link = entity::users_items::ActiveModel {
                user_id: Set(channel_user.user_id),
                channel_id: Set(channel_id),
                item_id: Set(item.id),
                read: Set(false),
                starred: Set(false),
            };
            link.insert(self.db.as_ref()).await?;
        }

        Ok(item)
    }

    pub async fn get_items_of_channel(
        &self,
        chan_id: i32,
        page: usize,
        page_size: usize,
    ) -> Result<PagedResult<items::Model>, ApiError> {
        log::debug!("Getting items of channel {}", chan_id);

        let item_paginator = Item::find()
            .filter(items::Column::ChannelId.eq(chan_id))
            .order_by_desc(items::Column::Id)
            .paginate(self.db.as_ref(), page_size);

        let total_items = item_paginator.num_items().await?;
        // Calling .num_pages() on the paginator re-query the database for the number of items
        // so we better do it ourself by reusing the .num_items() result
        let total_pages = (total_items / page_size) + (total_items % page_size > 0) as usize;
        let content = item_paginator.fetch_page(page - 1).await?;
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

    pub async fn get_all_items_of_channel(
        &self,
        chan_id: i32,
    ) -> Result<Vec<items::Model>, ApiError> {
        log::debug!("Getting items paginator of channel {}", chan_id);

        Ok(Item::find()
            .filter(items::Column::ChannelId.eq(chan_id))
            .order_by_desc(items::Column::Id)
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

        let item_paginator = Item::find()
            .join(JoinType::RightJoin, items::Relation::UsersItems.def())
            .filter(users_items::Column::UserId.eq(user_id))
            .order_by_desc(items::Column::Id)
            .paginate(self.db.as_ref(), page_size);

        let total_items = item_paginator.num_items().await?;
        // Calling .num_pages() on the paginator re-query the database for the number of items
        // so we better do it ourself by reusing the .num_items() result
        let total_pages = (total_items / page_size) + (total_items % page_size > 0) as usize;
        let content = item_paginator.fetch_page(page - 1).await?;
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

    /// # Select all the users linked to a channel
    /// TODO: I only need the user_id here
    async fn get_users_of_channel(
        &self,
        channel_id: i32,
    ) -> Result<Vec<channel_users::Model>, ApiError> {
        Ok(ChannelUsers::find()
            .filter(channel_users::Column::ChannelId.eq(channel_id))
            .all(self.db.as_ref())
            .await?)
    }
}
