use chrono::Utc;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::sea_query::{Alias, Expr, SimpleExpr};
use sea_orm::{entity::*, query::*, DbErr};
use sea_orm::{DeriveColumn, EnumIter};

use entity::channel_users::Entity as ChannelUsers;
use entity::channels::{Entity as Channel, Model};
use entity::prelude::ChannelsErrors;
use entity::users_items::Entity as UsersItems;
use entity::{channel_users, channels, channels_errors, users_items};

use crate::model::{HttpChannel, HttpChannelError, HttpNewChannel, HttpUserChannel, PagedResult};
use crate::services::rss::check_feed;
use crate::services::ServiceError;

pub struct ChannelService;

/// Generate a select statement for channel and user
fn user_channel_select_statement() -> Select<Channel> {
    Channel::find()
        .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
        .join(JoinType::LeftJoin, channels::Relation::UsersItems.def())
        .column_as(users_items::Column::ItemId.count(), "items_count")
        .column_as(
            Expr::expr(
                Into::<SimpleExpr>::into(Expr::col(users_items::Column::Read))
                    .cast_as(Alias::new("integer")),
            )
            .sum(),
            "items_read",
        )
}

impl ChannelService {
    #[tracing::instrument(skip(db))]
    pub async fn select_errors_by_chan_id<C>(
        db: &C,
        chan_id: i32,
    ) -> Result<Vec<HttpChannelError>, ServiceError>
    where
        C: ConnectionTrait,
    {
        Ok(ChannelsErrors::find()
            .join(JoinType::Join, channels_errors::Relation::Channels.def())
            .column_as(channels::Column::Name, "channel_name")
            .filter(channels_errors::Column::ChannelId.eq(chan_id))
            .into_model::<HttpChannelError>()
            .all(db)
            .await?)
    }

    #[tracing::instrument(skip(db))]
    pub async fn select_by_id_and_user_id<C>(
        db: &C,
        chan_id: i32,
        user_id: i32,
    ) -> Result<Option<HttpUserChannel>, ServiceError>
    where
        C: ConnectionTrait,
    {
        Ok(user_channel_select_statement()
            .filter(channel_users::Column::UserId.eq(user_id))
            .filter(channel_users::Column::ChannelId.eq(chan_id))
            .group_by(channels::Column::Id)
            .into_model::<HttpUserChannel>()
            .one(db)
            .await?)
    }

    #[tracing::instrument(skip(db))]
    pub async fn mark_channel_as_read<C>(db: &C, chan_id: i32, user_id: i32) -> Result<(), DbErr>
    where
        C: ConnectionTrait,
    {
        UsersItems::update_many()
            .col_expr(users_items::Column::Read, Expr::value(true))
            .filter(users_items::Column::ChannelId.eq(chan_id))
            .filter(users_items::Column::UserId.eq(user_id))
            .exec(db)
            .await?;

        tracing::debug!("Chanel {} marked as read for user {}", chan_id, user_id);

        Ok(())
    }

    ///  Select all the channels of a user, along side the total number of items
    #[tracing::instrument(skip(db))]
    pub async fn select_page_by_user_id<C>(
        db: &C,
        u_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<PagedResult<HttpUserChannel>, ServiceError>
    where
        C: ConnectionTrait,
    {
        let channel_pagination = user_channel_select_statement()
            .filter(channel_users::Column::UserId.eq(u_id))
            .group_by(channels::Column::Id)
            .group_by(channel_users::Column::RegistrationTimestamp)
            .order_by_desc(channel_users::Column::RegistrationTimestamp)
            .into_model::<HttpUserChannel>()
            .paginate(db, page_size);

        let total_items_and_pages = channel_pagination.num_items_and_pages().await?;
        let total_pages = total_items_and_pages.number_of_pages;
        let content = channel_pagination.fetch_page(page - 1).await?;
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

    /// # Create a new channel in the database
    #[tracing::instrument(skip(db))]
    async fn create_new_channel<C>(
        db: &C,
        new_channel: &HttpNewChannel,
    ) -> Result<Model, ServiceError>
    where
        C: ConnectionTrait,
    {
        // Check that the feed is a parsable RSS feed
        check_feed(&new_channel.url).await?;

        let channel = channels::ActiveModel {
            id: NotSet,
            name: Set(new_channel.name.to_owned()),
            url: Set(new_channel.url.to_owned()),
            last_update: NotSet,
            registration_timestamp: Set(Utc::now().into()),
            disabled: Set(false),
            failure_count: Set(0),
        };

        Ok(channel.insert(db).await?)
    }

    /// Create or linked an existing channel to a user
    #[tracing::instrument(skip(db))]
    pub async fn create_or_link_channel<C>(
        db: &C,
        new_channel: HttpNewChannel,
        other_user_id: i32,
    ) -> Result<Model, ServiceError>
    where
        C: ConnectionTrait,
    {
        let channel = match Channel::find()
            .filter(channels::Column::Url.eq(&*new_channel.url))
            .one(db)
            .await?
        {
            Some(found) => found,
            None => ChannelService::create_new_channel(db, &new_channel).await?,
        };

        let channel_user = channel_users::ActiveModel {
            channel_id: Set(channel.id),
            user_id: Set(other_user_id),
            registration_timestamp: Set(Utc::now().into()),
        };

        match channel_user.insert(db).await {
            Ok(_) => Ok(channel),
            Err(DbErr::Query(x)) => {
                tracing::error!(
                    "Channel {} for user {} already inserted: {x}",
                    channel.name,
                    other_user_id
                );
                Ok(channel)
            }
            Err(x) => Err(x.into()),
        }
    }

    /// Enable a channel and reset it's failure count
    #[tracing::instrument(skip(db))]
    pub async fn enable_channel<C>(db: &C, id: i32) -> Result<(), DbErr>
    where
        C: ConnectionTrait,
    {
        Channel::update_many()
            .col_expr(channels::Column::Disabled, Expr::value(false))
            .col_expr(channels::Column::FailureCount, Expr::value(0))
            .filter(channels::Column::Id.eq(id))
            .exec(db)
            .await?;

        tracing::debug!("Chanel {} enabled", id);

        Ok(())
    }

    /// Disable channels whom failure count is higher than the given threshold
    #[tracing::instrument(skip(db))]
    pub async fn disable_channels<C>(db: &C, threshold: u32) -> Result<(), DbErr>
    where
        C: ConnectionTrait,
    {
        let disabled_channels: UpdateResult = Channel::update_many()
            .col_expr(channels::Column::Disabled, Expr::value(true))
            .filter(channels::Column::FailureCount.eq(threshold))
            .filter(channels::Column::Disabled.eq(false))
            .exec(db)
            .await?;

        tracing::debug!("Disabled {} channels", disabled_channels.rows_affected);

        Ok(())
    }

    /// Return the list of user IDs of of a given channel
    #[tracing::instrument(skip(db))]
    pub async fn get_user_ids_of_channel<C>(db: &C, channel_id: i32) -> Result<Vec<i32>, DbErr>
    where
        C: ConnectionTrait,
    {
        ChannelUsers::find()
            .select_only()
            .column(channel_users::Column::UserId)
            .filter(channel_users::Column::ChannelId.eq(channel_id))
            .into_values::<_, QueryAs>()
            .all(db)
            .await
    }

    /// Return the list of all enabled channels
    #[tracing::instrument(skip(db))]
    pub async fn get_all_enabled_channels<C>(db: &C) -> Result<Vec<Model>, DbErr>
    where
        C: ConnectionTrait,
    {
        Channel::find()
            .filter(channels::Column::Disabled.eq(false))
            .all(db)
            .await
    }

    /// Update the last fetched timestamp of a channel
    #[tracing::instrument(skip(db))]
    pub async fn update_last_fetched<C>(
        db: &C,
        channel_id: i32,
        date: DateTimeWithTimeZone,
    ) -> Result<(), DbErr>
    where
        C: ConnectionTrait,
    {
        Channel::update_many()
            .col_expr(channels::Column::LastUpdate, Expr::value(date))
            .filter(channels::Column::Id.eq(channel_id))
            .exec(db)
            .await?;

        Ok(())
    }

    /// Update the failure count of the given channel and insert the error in the dedicated table
    #[tracing::instrument(skip(connection))]
    pub async fn fail_channel<C>(
        connection: &C,
        channel_id: i32,
        error_cause: &str,
    ) -> Result<(), DbErr>
    where
        C: ConnectionTrait,
    {
        Channel::update_many()
            .col_expr(
                channels::Column::FailureCount,
                Expr::col(channels::Column::FailureCount).add(1),
            )
            .filter(channels::Column::Id.eq(channel_id))
            .exec(connection)
            .await?;

        let channel_error = channels_errors::ActiveModel {
            id: NotSet,
            channel_id: Set(channel_id),
            error_reason: Set(Some(error_cause.to_owned())),
            error_timestamp: Set(Utc::now().into()),
        };

        channel_error.insert(connection).await?;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryAs {
    UserId,
}
