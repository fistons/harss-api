use chrono::{DateTime, Utc};
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*, DbErr};

use entity::channels::Entity as Channel;
use entity::{channel_users, channels, users_items};

use crate::errors::ApiError;
use crate::model::{HttpChannel, HttpNewChannel, HttpUserChannel, PagedResult};

#[derive(Clone)]
pub struct ChannelService {
    db: DatabaseConnection,
}

impl ChannelService {
    pub fn new(db: DatabaseConnection) -> Self {
        ChannelService { db }
    }

    /// # Select a channel by id and user id
    /// We set the user_id by security, to prevent users trying to get channels the shouldn't have
    /// access
    pub async fn select_by_id_and_user_id(
        &self,
        u_id: i32,
        chan_id: i32,
    ) -> Result<Option<HttpChannel>, ApiError> {
        Ok(Channel::find()
            .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
            .filter(channel_users::Column::ChannelId.eq(chan_id))
            .filter(channel_users::Column::UserId.eq(u_id))
            .into_model::<HttpChannel>()
            .one(&self.db)
            .await?)
    }

    ///  Select all the channels of a user, along side the total number of items
    pub async fn select_page_by_user_id(
        &self,
        u_id: i32,
        page: usize,
        page_size: usize,
    ) -> Result<PagedResult<HttpUserChannel>, ApiError> {
        let channel_paginator = Channel::find()
            .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
            .join(JoinType::LeftJoin, channels::Relation::UsersItems.def())
            .column_as(users_items::Column::ItemId.count(), "items_count")
            .column_as(
                Expr::expr(
                    Expr::col(users_items::Column::Read)
                        .into_simple_expr()
                        .cast_as(Alias::new("integer")),
                )
                .sum(),
                "items_read",
            )
            .filter(channel_users::Column::UserId.eq(u_id))
            .group_by(channels::Column::Id)
            .into_model::<HttpUserChannel>()
            .paginate(&self.db, page_size);

        let total_items = channel_paginator.num_items().await?;
        // Calling .num_pages() on the paginator re-query the database for the number of items
        // so we better do it ourself by reusing the .num_items() result
        let total_pages = (total_items / page_size) + (total_items % page_size > 0) as usize;
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
    pub async fn select_all(&self) -> Result<Vec<HttpChannel>, ApiError> {
        Ok(Channel::find()
            .into_model::<HttpChannel>()
            .all(&self.db)
            .await?)
    }

    pub async fn select_all_by_user_id(&self, user_id: i32) -> Result<Vec<HttpChannel>, ApiError> {
        Ok(Channel::find()
            .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
            .filter(channel_users::Column::UserId.eq(user_id))
            .into_model::<HttpChannel>()
            .all(&self.db)
            .await?)
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
            last_update: NotSet,
        };

        Ok(channel.insert(&self.db).await?)
    }

    /// Create or linked an existing channel to a user
    pub async fn create_or_link_channel(
        &self,
        new_channel: HttpNewChannel,
        other_user_id: i32,
    ) -> Result<channels::Model, ApiError> {
        let channel = match Channel::find()
            .filter(channels::Column::Url.eq(&*new_channel.url))
            .one(&self.db)
            .await?
        {
            Some(found) => found,
            None => self.create_new_channel(&new_channel).await?,
        };

        let channel_user = channel_users::ActiveModel {
            channel_id: Set(channel.id),
            user_id: Set(other_user_id),
        };

        match channel_user.insert(&self.db).await {
            Ok(_) => Ok(channel),
            Err(DbErr::Query(x)) => {
                log::warn!(
                    "Channel {} for user {} already inserted: {x}",
                    channel.name,
                    other_user_id
                );
                Ok(channel)
            }
            Err(x) => Err(x.into()),
        }
    }

    /// Update the last fetched timestamp of a channel
    pub async fn update_last_fetched(
        &self,
        channel_id: i32,
        date: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        Channel::update_many()
            .col_expr(channels::Column::LastUpdate, Expr::value(date))
            .filter(channels::Column::Id.eq(channel_id))
            .exec(&self.db)
            .await?;

        Ok(())
    }
}
