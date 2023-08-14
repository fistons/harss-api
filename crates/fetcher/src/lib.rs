extern crate core;

use std::collections::HashSet;
use std::error::Error;

use anyhow::Context;
use chrono::Utc;
use deadpool_redis::{Pool as Redis, PoolError};
use feed_rs::model::{Entry, Feed};
use once_cell::sync::Lazy;
use redis::aio::Connection;
use redis::{AsyncCommands, ExistenceCheck, RedisError, RedisResult, SetExpiry, SetOptions};
use reqwest::Client;
use uuid::Uuid;

use common::channels::*;
use common::items::*;
use common::model::{Channel, NewItem};
use common::{DbError, Pool};

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("rss-aggregator checker (+https://github.com/fistons/rss-aggregator)")
        .build()
        .expect("Could not build CLIENT")
});

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Redis error: {0}")]
    RedisError(#[from] RedisError),
    #[error("Redis Pool error: {0}")]
    PoolError(#[from] PoolError),
    #[error("Database error: {0}")]
    SqlError(#[from] DbError),
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
pub async fn process(connection: &Pool, redis: &Redis) -> Result<(), anyhow::Error> {
    let channels = get_all_enabled_channels(connection)
        .await
        .context("Could not get channels to update")?;
    let mut redis = redis.get().await?;

    for channel in channels {
        let (key, value, response) = acquire_lock(&mut redis, channel.id).await;
        if response?.is_none() {
            tracing::error!(
                "Lock for channel {} already acquired. Giving up for now",
                channel.name
            );
            continue;
        }

        if let Err(error) = update_channel(connection, channel).await {
            tracing::error!("{:?}", error.source());
        }

        // Remove the lock on the channel.
        release_lock(&mut redis, &key, &value).await?;
    }

    let threshold = std::env::var("FAILURE_THRESHOLD")
        .map(|x| x.parse::<u32>().unwrap_or(3))
        .unwrap_or(3);

    // Disable all the channels where the failed count is a higher than FAILURE_THRESHOLD.
    // If FAILURE_THRESHOLD = 0, don't do anything
    if threshold > 0 {
        disable_channels(connection, threshold).await?;
    }

    Ok(())
}

async fn acquire_lock(
    redis: &mut Connection,
    channel_id: i32,
) -> (String, String, RedisResult<Option<String>>) {
    let value = Uuid::new_v4().to_string();
    let key = format!("lock.channel.{}", channel_id);

    let options = SetOptions::default()
        .conditional_set(ExistenceCheck::NX)
        .with_expiration(SetExpiry::EX(60));

    let redis_result = redis.set_options(&key, &value, options).await;

    (key, value, redis_result)
}

async fn release_lock(redis: &mut Connection, key: &str, value: &str) -> RedisResult<()> {
    let redis_value = redis.get::<&str, Option<String>>(key).await?;
    if redis_value.unwrap() == value {
        redis.del(key).await?;
    }

    Ok(())
}

#[tracing::instrument(skip(connection))]
async fn update_channel(connection: &Pool, channel: Channel) -> Result<(), FetchError> {
    tracing::info!("Updating {} ({})", channel.name, channel.url);

    let feed = match get_and_parse_feed(&channel.url).await {
        Ok(feed) => feed,
        Err(error) => {
            fail_channel(connection, channel.id, &error.to_string()).await?;
            return Err(error);
        }
    };

    let current_item_ids = fetch_current_items_id(connection, &channel).await?;

    // Retrieve all the items not already retrieved in precedent run
    let new_items = feed
        .entries
        .into_iter()
        .filter(|item| !current_item_ids.contains(&item.id))
        .map(|entry| item_from_rss_entry(entry, channel.id))
        .collect::<Vec<NewItem>>();

    // Retrieve all the users that have subscribe to the channel
    let user_ids = get_user_ids_of_channel(connection, channel.id).await?;

    // For each new item, create the entry in the database, and add it to each user
    //FIXME Insert the delta only, not only the new articles
    for item in new_items {
        insert_item_for_user(connection, &item, &user_ids).await?;
    }

    update_last_fetched(connection, channel.id, Utc::now()).await?;

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
async fn fetch_current_items_id(
    connection: &Pool,
    channel: &Channel,
) -> Result<HashSet<String>, DbError>
where
{
    let items = get_all_items_guid_of_channel(connection, channel.id).await?;

    Ok(items.into_iter().flatten().collect::<HashSet<String>>())
}

/// Create an Item Entity from an RSS entry
fn item_from_rss_entry(entry: Entry, channel_id: i32) -> NewItem {
    let title = entry.title.map(|x| x.content);
    let guid = Some(entry.id);
    let url = entry.links.get(0).map(|x| String::from(&x.href[..]));
    let content = entry.summary.map(|x| x.content);
    let now = Utc::now();
    let publish_timestamp = entry.published.or(Some(now));

    NewItem {
        guid,
        title,
        url,
        content,
        fetch_timestamp: now,
        publish_timestamp,
        channel_id,
    }
}
