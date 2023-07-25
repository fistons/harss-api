extern crate core;

use std::collections::HashSet;
use std::error::Error;

use anyhow::Context;
use chrono::Utc;
use feed_rs::model::{Entry, Feed};
use once_cell::sync::Lazy;
use reqwest::Client;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*};

use entity::channels;
use rss_common::services::channels::ChannelService;
use rss_common::services::items::ItemService;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("rss-aggregator checker (+https://github.com/fistons/rss-aggregator)")
        .build()
        .expect("Could not build CLIENT")
});

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Database error: {0}")]
    SqlError(#[from] sea_orm::DbErr),
    #[error("Parsing error: {0}")]
    ParseError(#[from] feed_rs::parser::ParseFeedError),
    #[error("Could not fetch the feed: {0}")]
    GetError(#[from] reqwest::Error),
    #[error("HTTP status code error: Upstream feed returned HTTP status code {0}")]
    StatusCodeError(u16),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// Process the whole database update in one single transaction. BALLSY.
#[tracing::instrument(skip_all)]
pub async fn process(connection: &DatabaseConnection) -> Result<(), anyhow::Error> {
    let txn = connection.begin().await?;

    let channels = ChannelService::get_all_enabled_channels(&txn)
        .await
        .context("Could not get channels to update")?;

    for channel in channels {
        if let Err(error) = update_channel(&txn, channel).await {
            tracing::error!("{:?}", error.source());
        }
    }

    let threshold = std::env::var("FAILURE_THRESHOLD")
        .map(|x| x.parse::<u32>().unwrap_or(3))
        .unwrap_or(3);

    // Disable all the channels where the failed count is a higher than FAILURE_THRESHOLD.
    // If FAILURE_THRESHOLD = 0, don't do anything
    if threshold > 0 {
        ChannelService::disable_channels(&txn, threshold).await?;
    }

    txn.commit().await?;

    Ok(())
}

#[tracing::instrument(skip(connection))]
async fn update_channel<C>(connection: &C, channel: channels::Model) -> Result<(), FetchError>
where
    C: ConnectionTrait,
{
    let feed = match get_and_parse_feed(&channel.url).await {
        Ok(feed) => feed,
        Err(error) => {
            ChannelService::fail_channel(connection, channel.id, &error.to_string()).await?;
            return Err(error);
        }
    };

    let current_item_ids = fetch_current_items_id(connection, &channel).await?;

    let new_items = feed
        .entries
        .into_iter()
        .filter(|item| !current_item_ids.contains(&item.id))
        .map(|entry| item_from_rss_entry(entry, channel.id))
        .collect::<Vec<entity::items::ActiveModel>>();

    let user_ids = ChannelService::get_user_ids_of_channel(connection, channel.id).await?;
    for item in new_items {
        let item = item.insert(connection).await?;
        for user_id in &user_ids {
            build_user_channels(user_id, &channel.id, &item.id)
                .insert(connection)
                .await?;
        }
    }
    let now: DateTimeWithTimeZone = Utc::now().into();
    ChannelService::update_last_fetched(connection, channel.id, now).await?;

    Ok(())
}

/// Download and parse the feed of the given channel
async fn get_and_parse_feed(channel_url: &str) -> Result<Feed, FetchError> {
    let response = CLIENT.get(channel_url).send().await?;

    if !response.status().is_success() {
        return Err(FetchError::StatusCodeError(response.status().as_u16()));
    }

    let data = response.bytes().await?;

    Ok(feed_rs::parser::parse(&data[..])?)
}

/// Returns the list of all the registered item ids of a channel.
async fn fetch_current_items_id<C>(
    connection: &C,
    channel: &channels::Model,
) -> Result<HashSet<String>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let items = ItemService::get_all_items_of_channel(connection, channel.id).await?;

    Ok(items
        .into_iter()
        .filter_map(|x| x.guid)
        .collect::<HashSet<String>>())
}

fn build_user_channels(
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

/// Create an Item Entity from an RSS entry
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
