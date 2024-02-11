use crate::common::channels::{
    disable_channels, fail_channel, get_all_enabled_channels, get_last_update, update_last_fetched,
};
use crate::common::items::{insert_items, insert_items_delta_for_all_registered_users};
use crate::common::model::{Channel, NewItem};
use crate::common::DbError;
use anyhow::Context;
use chrono::{DateTime, Days, Utc};
use deadpool_redis::{Connection, Pool as RedisPool, PoolError};
use feed_rs::model::{Entry, Feed};
use once_cell::sync::Lazy;
use redis::{AsyncCommands, ExistenceCheck, RedisError, RedisResult, SetExpiry, SetOptions};
use reqwest::Client;
use sqlx::PgPool;
use std::error::Error;
use tracing::{info, Instrument};
use uuid::Uuid;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("HaRSS fetcher (+https://github.com/fistons/rss-aggregator)")
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

/// Check for RSS channel updates and proceed.
#[tracing::instrument(skip_all)]
pub async fn process(connection: &PgPool, redis: &RedisPool) -> Result<(), anyhow::Error> {
    let channels = get_all_enabled_channels(connection)
        .await
        .context("Could not get channels to update")?;

    for channel in channels {
        if let Err(error) = update_channel(connection, redis, &channel)
            .in_current_span()
            .await
        {
            tracing::error!("{:?}", error.source());
        }
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

#[tracing::instrument(skip(connection, redis))]
pub async fn update_channel(
    connection: &PgPool,
    redis: &RedisPool,
    channel: &Channel,
) -> Result<(), FetchError> {
    let mut redis = redis.get().await?;

    let (key, value, response) = acquire_lock(&mut redis, channel.id).await;
    if response?.is_none() {
        tracing::error!(
            "Lock for channel {} already acquired. Giving up for now",
            channel.name
        );
        return Ok(());
    }

    info!("Updating {} {} ({})", channel.id, channel.name, channel.url);

    let feed = match get_and_parse_feed(&channel.url).await {
        Ok(feed) => feed,
        Err(error) => {
            fail_channel(connection, channel.id, &error.to_string()).await?;
            return Err(error);
        }
    };

    let now = Utc::now();
    let last_update = get_last_update(connection, &channel.id)
        .await?
        .unwrap_or(Utc::now().checked_sub_days(Days::new(7)).unwrap());

    // Retrieve all the items not already retrieved in precedent run
    let new_items = feed
        .entries
        .into_iter()
        .map(|entry| item_from_rss_entry(entry, channel.id, &now))
        .filter(|item| item.publish_timestamp.unwrap_or(last_update) >= last_update)
        .collect::<Vec<NewItem>>();

    insert_items(connection, &new_items).await?;
    insert_items_delta_for_all_registered_users(connection, channel.id, &now).await?;
    update_last_fetched(connection, channel.id, &now).await?;

    release_lock(&mut redis, &key, &value).await?;

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

/// Download and parse the feed of the given channel
async fn get_and_parse_feed(channel_url: &str) -> Result<Feed, FetchError> {
    let response = CLIENT.get(channel_url).send().await?;

    if !response.status().is_success() {
        return Err(FetchError::StatusCodeError(response.status().as_u16()));
    }

    let data = response.bytes().await?;

    Ok(feed_rs::parser::parse(&data[..])?)
}

/// Create an Item Entity from an RSS entry
fn item_from_rss_entry(entry: Entry, channel_id: i32, timestamp: &DateTime<Utc>) -> NewItem {
    let title = entry.title.map(|x| x.content);
    let guid = Some(entry.id);
    let url = entry.links.first().map(|x| String::from(&x.href[..]));
    let content = entry.summary.map(|x| x.content);
    let publish_timestamp = entry.published.or(Some(*timestamp));

    NewItem {
        guid,
        title,
        url,
        content,
        fetch_timestamp: *timestamp,
        publish_timestamp,
        channel_id,
    }
}
