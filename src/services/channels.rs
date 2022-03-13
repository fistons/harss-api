use std::sync::Arc;

use sea_orm::{entity::*, query::*};
use sea_orm::DatabaseConnection;

use entity::{channel_users, channels};
use entity::channels::Entity as Channel;

use crate::errors::ApiError;
use crate::model::{HttpNewChannel, PagedResult};

#[derive(Clone)]
pub struct ChannelService {
    db: Arc<DatabaseConnection>,
}

impl ChannelService {
    pub fn new(db: DatabaseConnection) -> Self {
        ChannelService {
            db: Arc::new(db)
        }
    }

    /// # Select a channel by id and user id
    /// We set the user_id by security, to prevent users trying to get channels the shouldn't have
    /// access
    pub async fn select_by_id_and_user_id(
        &self,
        u_id: i32,
        chan_id: i32,
    ) -> Result<Option<channels::Model>, ApiError> {
        Ok(Channel::find()
            .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
            .filter(channel_users::Column::ChannelId.eq(chan_id))
            .filter(channel_users::Column::UserId.eq(u_id))
            .one(self.db.as_ref())
            .await?)
    }

    /// # Select all the channels of a user
    pub async fn select_all_by_user_id(&self, u_id: i32, page: usize, page_size: usize) -> Result<PagedResult<channels::Model>, ApiError> {
        
        let channel_paginator = Channel::find()
            .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
            .filter(channel_users::Column::UserId.eq(u_id))
            .paginate(self.db.as_ref(), page_size);

        let total_pages = channel_paginator.num_pages().await?;
        let total_items = channel_paginator.num_items().await?;
        let content = channel_paginator.fetch_page(page - 1).await?;
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

    /// # Select all the channels
    pub async fn select_all(&self) -> Result<Vec<channels::Model>, ApiError> {
        Ok(Channel::find().all(self.db.as_ref()).await?)
    }


    /// # Create a new channel in the database
    async fn create_new_channel(
        &self,
        new_channel: &HttpNewChannel,
    ) -> Result<channels::Model, ApiError> {
        let channel = channels::ActiveModel {
            id: NotSet,
            name: Set(new_channel.name.to_owned()),
            url: Set(new_channel.url.to_owned()),
        };

        Ok(channel.insert(self.db.as_ref()).await?)
    }


    pub async fn create_or_link_channel(
        &self,
        new_channel: HttpNewChannel,
        other_user_id: i32,
    ) -> Result<channels::Model, ApiError> {
        let channel = match Channel::find()
            .filter(channels::Column::Url.eq(&*new_channel.url))
            .one(self.db.as_ref()).await?
        {
            Some(found) => found,
            None => self.create_new_channel(&new_channel).await?,
        };

        let channel_user = channel_users::ActiveModel {
            channel_id: Set(channel.id),
            user_id: Set(other_user_id),
        };

        channel_user.insert(self.db.as_ref()).await?;

        Ok(channel)
    }
}
