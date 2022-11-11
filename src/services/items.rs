use sea_orm::sea_query::Expr;
use sea_orm::{entity::*, query::*, DeriveColumn, EnumIter};
use sea_orm::{DatabaseConnection, DbErr};

use entity::channels;
use entity::items;
use entity::items::Entity as Item;
use entity::prelude::UsersItems;
use entity::users_items;

use crate::model::{HttpUserItem, PagedResult};

#[derive(Clone)]
pub struct ItemService {
    db: DatabaseConnection,
}

impl ItemService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_items_of_channel(
        &self,
        chan_id: i32,
        user_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<PagedResult<HttpUserItem>, DbErr> {
        let item_paginator = Item::find()
            .join(JoinType::RightJoin, items::Relation::UsersItems.def())
            .join(JoinType::RightJoin, items::Relation::Channels.def())
            .column_as(users_items::Column::Read, "read")
            .column_as(users_items::Column::Starred, "starred")
            .column_as(channels::Column::Name, "channel_name")
            .column_as(channels::Column::Id, "channel_id")
            .filter(users_items::Column::ChannelId.eq(chan_id))
            .filter(users_items::Column::UserId.eq(user_id))
            .order_by_desc(items::Column::PublishTimestamp)
            .into_model::<HttpUserItem>()
            .paginate(&self.db, page_size);

        let total_items_and_pages = item_paginator.num_items_and_pages().await?;
        let total_pages = total_items_and_pages.number_of_pages;
        let content = item_paginator.fetch_page(page - 1).await?;
        let elements_number = content.len();

        Ok(PagedResult {
            content,
            page,
            page_size,
            total_pages,
            elements_number,
            total_items: total_items_and_pages.number_of_items,
        })
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_all_items_of_channel(&self, chan_id: i32) -> Result<Vec<items::Model>, DbErr> {
        Item::find()
            .filter(items::Column::ChannelId.eq(chan_id))
            .order_by_desc(items::Column::PublishTimestamp)
            .all(&self.db)
            .await
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_items_of_user(
        &self,
        user_id: i32,
        page: u64,
        page_size: u64,
        read: Option<bool>,
        starred: Option<bool>,
    ) -> Result<PagedResult<HttpUserItem>, DbErr> {
        let mut query = Item::find()
            .join(JoinType::RightJoin, items::Relation::UsersItems.def())
            .join(JoinType::RightJoin, items::Relation::Channels.def())
            .column_as(channels::Column::Name, "channel_name")
            .column_as(channels::Column::Id, "channel_id")
            .column(users_items::Column::Read)
            .column(users_items::Column::Starred)
            .filter(users_items::Column::UserId.eq(user_id));

        if let Some(r) = read {
            query = query.filter(users_items::Column::Read.eq(r))
        }

        if let Some(s) = starred {
            query = query.filter(users_items::Column::Starred.eq(s))
        }

        let item_paginator = query
            .order_by_desc(items::Column::PublishTimestamp)
            .into_model::<HttpUserItem>()
            .paginate(&self.db, page_size);

        let total_items_and_pages = item_paginator.num_items_and_pages().await?;
        let total_pages = total_items_and_pages.number_of_pages;
        let content = item_paginator.fetch_page(page - 1).await?;
        let elements_number = content.len();

        Ok(PagedResult {
            content,
            page,
            page_size,
            total_pages,
            elements_number,
            total_items: total_items_and_pages.number_of_items,
        })
    }

    /// Update the read status of an item for a given user
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn set_item_read(
        &self,
        user_id: i32,
        ids: Vec<i32>,
        read: bool,
    ) -> Result<(), DbErr> {
        self.update_column(user_id, ids, users_items::Column::Read, read)
            .await?;

        Ok(())
    }

    /// Update the read status of an item for a given user
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn set_item_starred(
        &self,
        user_id: i32,
        ids: Vec<i32>,
        starred: bool,
    ) -> Result<(), DbErr> {
        self.update_column(user_id, ids, users_items::Column::Starred, starred)
            .await?;

        Ok(())
    }

    /// Update the given column with given value for the given user and items ids.
    async fn update_column(
        &self,
        user_id: i32,
        ids: Vec<i32>,
        column: users_items::Column,
        value: bool,
    ) -> Result<(), DbErr> {
        UsersItems::update_many()
            .col_expr(column, Expr::value(value))
            .filter(users_items::Column::UserId.eq(user_id))
            .filter(users_items::Column::ItemId.is_in(ids))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryAs {
    UserId,
}
