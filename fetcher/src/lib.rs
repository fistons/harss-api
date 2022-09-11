use anyhow::Context;
use std::collections::HashSet;
use std::error::Error;

use chrono::Utc;
use feed_rs::model::Entry;
use reqwest::Client;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*, DeriveColumn, EnumIter};
use tracing::Instrument;

use entity::channel_users::Entity as ChannelUser;
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

    #[tracing::instrument(skip_all)]
    pub async fn fetch(&self) -> Result<(), anyhow::Error> {
        let channels = Channel::find()
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
            if let Err(meh) = task.await {
                tracing::error!("{:?}", meh.source());
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update_channel(&self, channel: entity::channels::Model) -> Result<(), FetchError> {
        let response = self.client.get(&channel.url).send().await?;

        let data = response.bytes().await?;
        let feed = feed_rs::parser::parse(&data[..])?;
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
        txn.commit().await?;

        Ok(())
    }

    async fn fetch_current_items_id(
        &self,
        channel: &entity::channels::Model,
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
