extern crate core;

use std::collections::HashSet;
use std::error::Error;

use anyhow::Context;
use chrono::Utc;
use feed_rs::model::{Entry, Feed};
use reqwest::Client;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::sea_query::Expr;
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*, DeriveColumn, EnumIter};
use tracing::Instrument;

use entity::channel_users::Entity as ChannelUser;
use entity::channels;
use entity::channels::Entity as Channel;
use entity::items::Entity as Item;

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Database error: {0}")]
    SqlError(#[from] sea_orm::DbErr),
    #[error("Parsing error: {0}")]
    ParseError(#[from] feed_rs::parser::ParseFeedError),
    #[error("Could not fetch the feed: {0}")]
    GetError(#[from] reqwest::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct Fetcher {
    client: Client,
    pool: DatabaseConnection,
}

impl Fetcher {
    pub fn new(client: Client, pool: DatabaseConnection) -> Self {
        Self { client, pool }
    }

    #[tracing::instrument(skip_all, level = "debug")]
    pub async fn fetch(&self) -> Result<(), anyhow::Error> {
        let channels = Channel::find()
            .filter(channels::Column::Disabled.eq(false))
            .all(&self.pool)
            .await
            .context("Could not get channels to update")?;

        let mut tasks = vec![];
        for channel in channels {
            let clone = self.clone();
            let future = async move { clone.update_channel(channel).await };
            let task = tokio::task::spawn(future.in_current_span());

            tasks.push(task);
        }

        for task in tasks {
            if let Err(error) = task.await {
                tracing::error!("{:?}", error.source());
            }
        }

        self.disable_channels().await?;

        Ok(())
    }

    #[tracing::instrument(skip(self), level = "debug")]
    async fn update_channel(&self, channel: channels::Model) -> Result<(), FetchError> {
        let feed = match self.get_and_parse_feed(&channel).await {
            Ok(feed) => feed,
            Err(error) => {
                self.fail_channels(channel.id).await?;
                return Err(error);
            }
        };

        let current_items = self.fetch_current_items_id(&channel).await?;

        let new_items = feed
            .entries
            .into_iter()
            .filter(|item| !current_items.contains(&item.id))
            .map(|entry| item_from_rss_entry(entry, channel.id))
            .collect::<Vec<entity::items::ActiveModel>>();

        let user_ids = self.get_users_of_channel(channel.id).await?;
        let txn = self.pool.begin().await?;
        for item in new_items {
            let item = item.insert(&txn).await?;
            for user_id in &user_ids {
                self.build_user_channels(user_id, &channel.id, &item.id)
                    .insert(&txn)
                    .await?;
            }
        }
        self.update_channel_timestamp(&channel.id, &txn).await?;
        txn.commit().await?;

        Ok(())
    }

    async fn get_and_parse_feed(&self, channel: &channels::Model) -> Result<Feed, FetchError> {
        let response = self.client.get(&channel.url).send().await?;
        let data = response.bytes().await?;

        Ok(feed_rs::parser::parse(&data[..])?)
    }

    #[tracing::instrument(skip(self, txn), level = "debug")]
    async fn update_channel_timestamp<C>(&self, channel_id: &i32, txn: &C) -> Result<(), FetchError>
    where
        C: ConnectionTrait,
    {
        let now: DateTimeWithTimeZone = Utc::now().into();
        Channel::update_many()
            .col_expr(channels::Column::LastUpdate, Expr::value(now))
            .filter(channels::Column::Id.eq(*channel_id))
            .exec(txn)
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self), level = "debug")]
    async fn fail_channels(&self, channel_id: i32) -> Result<(), FetchError> {
        Channel::update_many()
            .col_expr(
                channels::Column::FailureCount,
                Expr::col(channels::Column::FailureCount).add(1),
            )
            .filter(channels::Column::Id.eq(channel_id))
            .exec(&self.pool)
            .await?;
        Ok(())
    }

    /// Disable all the channels where the failed count is a multiple of FAILURE_THRESHOLD.
    /// If FAILURE_THRESHOLD = 0, don't do anything
    #[tracing::instrument(skip(self), level = "debug")]
    async fn disable_channels(&self) -> Result<(), FetchError> {
        let threshold = std::env::var("FAILURE_THRESHOLD")
            .map(|x| x.parse::<u32>().unwrap_or(3))
            .unwrap_or(3);

        if threshold > 0 {
            let disabled_channels: UpdateResult = Channel::update_many()
                .col_expr(channels::Column::Disabled, Expr::value(true))
                .filter(channels::Column::FailureCount.eq(threshold))
                .filter(channels::Column::Disabled.eq(false))
                .exec(&self.pool)
                .await?;

            tracing::debug!("Disabled {} channels", disabled_channels.rows_affected);
        }
        Ok(())
    }

    async fn fetch_current_items_id(
        &self,
        channel: &channels::Model,
    ) -> Result<HashSet<String>, sea_orm::DbErr> {
        let items: Vec<entity::items::Model> = Item::find()
            .filter(entity::items::Column::ChannelId.eq(channel.id))
            .all(&self.pool)
            .await?;

        Ok(items
            .into_iter()
            .filter_map(|x| x.guid)
            .collect::<HashSet<String>>())
    }

    async fn get_users_of_channel(&self, channel_id: i32) -> Result<Vec<i32>, sea_orm::DbErr> {
        ChannelUser::find()
            .select_only()
            .column(entity::channel_users::Column::UserId)
            .filter(entity::channel_users::Column::ChannelId.eq(channel_id))
            .into_values::<_, QueryAs>()
            .all(&self.pool)
            .await
    }

    fn build_user_channels(
        &self,
        user_id: &i32,
        chan_id: &i32,
        item_id: &i32,
    ) -> entity::users_items::ActiveModel {
        entity::users_items::ActiveModel {
            user_id: Set(*user_id),
            channel_id: Set(*chan_id),
            item_id: Set(*item_id),
            read: Set(false),
            starred: Set(false),
        }
    }
}

fn item_from_rss_entry(entry: Entry, channel_id: i32) -> entity::items::ActiveModel {
    let title = entry.title.map(|x| x.content);
    let guid = Some(entry.id);
    let url = entry.links.get(0).map(|x| String::from(&x.href[..]));
    let content = entry.summary.map(|x| x.content);
    let now: DateTimeWithTimeZone = Utc::now().into();
    entity::items::ActiveModel {
        id: NotSet,
        guid: Set(guid),
        title: Set(title),
        url: Set(url),
        content: Set(content),
        fetch_timestamp: Set(now),
        publish_timestamp: Set(entry.published.map(|x| x.into()).or(Some(now))),
        channel_id: Set(channel_id),
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryAs {
    UserId,
}
