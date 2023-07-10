extern crate core;

use std::collections::HashSet;
use std::error::Error;

use anyhow::Context;
use chrono::Utc;
use feed_rs::model::{Entry, Feed};
use once_cell::sync::Lazy;
use reqwest::Client;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::sea_query::Expr;
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*, DeriveColumn, EnumIter};

use entity::channel_users::Entity as ChannelUser;
use entity::channels;
use entity::channels::Entity as Channel;
use entity::channels_errors;
use entity::items::Entity as Item;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("rss-aggregator checker (+https://github.com/fistons/rss-aggregator)")
        .build()
        .expect("Could not build client")
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

#[tracing::instrument(skip_all)]
pub async fn fetch(connection: &DatabaseConnection) -> Result<(), anyhow::Error> {
    let channels = Channel::find()
        .filter(channels::Column::Disabled.eq(false))
        .all(connection)
        .await
        .context("Could not get channels to update")?;

    for channel in channels {
        if let Err(error) = update_channel(connection, channel).await {
            tracing::error!("{:?}", error.source());
        }
    }

    disable_channels(connection).await?;

    Ok(())
}

#[tracing::instrument(skip(connection))]
async fn update_channel(
    connection: &DatabaseConnection,
    channel: channels::Model,
) -> Result<(), FetchError> {
    let feed = match get_and_parse_feed(&channel).await {
        Ok(feed) => feed,
        Err(error) => {
            fail_channels(connection, channel.id, &error.to_string()).await?;
            return Err(error);
        }
    };

    let current_items = fetch_current_items_id(connection, &channel).await?;

    let new_items = feed
        .entries
        .into_iter()
        .filter(|item| !current_items.contains(&item.id))
        .map(|entry| item_from_rss_entry(entry, channel.id))
        .collect::<Vec<entity::items::ActiveModel>>();

    let user_ids = get_users_of_channel(connection, channel.id).await?;
    let txn = connection.begin().await?;
    for item in new_items {
        let item = item.insert(&txn).await?;
        for user_id in &user_ids {
            build_user_channels(user_id, &channel.id, &item.id)
                .insert(&txn)
                .await?;
        }
    }
    update_channel_timestamp(&channel.id, &txn).await?;
    txn.commit().await?;

    Ok(())
}

/// Download and parse the feed of the given channel
async fn get_and_parse_feed(channel: &channels::Model) -> Result<Feed, FetchError> {
    let response = CLIENT.get(&channel.url).send().await?;

    if !response.status().is_success() {
        return Err(FetchError::StatusCodeError(response.status().as_u16()));
    }

    let data = response.bytes().await?;

    Ok(feed_rs::parser::parse(&data[..])?)
}

#[tracing::instrument(skip(txn))]
async fn update_channel_timestamp<C>(channel_id: &i32, txn: &C) -> Result<(), FetchError>
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

/// Update the failure count of the given channel and insert the error in the dedicated table
#[tracing::instrument(skip(connection))]
async fn fail_channels(
    connection: &DatabaseConnection,
    channel_id: i32,
    error_cause: &str,
) -> Result<(), FetchError> {
    let txn = connection.begin().await?;
    Channel::update_many()
        .col_expr(
            channels::Column::FailureCount,
            Expr::col(channels::Column::FailureCount).add(1),
        )
        .filter(channels::Column::Id.eq(channel_id))
        .exec(&txn)
        .await?;

    let channel_error = channels_errors::ActiveModel {
        id: NotSet,
        channel_id: Set(channel_id),
        error_reason: Set(Some(error_cause.to_owned())),
        error_timestamp: Set(Utc::now().into()),
    };

    channel_error.insert(&txn).await?;

    txn.commit().await?;
    Ok(())
}

/// Disable all the channels where the failed count is a multiple of FAILURE_THRESHOLD.
/// If FAILURE_THRESHOLD = 0, don't do anything
#[tracing::instrument(skip(connection))]
async fn disable_channels(connection: &DatabaseConnection) -> Result<(), FetchError> {
    let threshold = std::env::var("FAILURE_THRESHOLD")
        .map(|x| x.parse::<u32>().unwrap_or(3))
        .unwrap_or(3);

    if threshold > 0 {
        let disabled_channels: UpdateResult = Channel::update_many()
            .col_expr(channels::Column::Disabled, Expr::value(true))
            .filter(channels::Column::FailureCount.eq(threshold))
            .filter(channels::Column::Disabled.eq(false))
            .exec(connection)
            .await?;

        tracing::debug!("Disabled {} channels", disabled_channels.rows_affected);
    }
    Ok(())
}

async fn fetch_current_items_id(
    connection: &DatabaseConnection,
    channel: &channels::Model,
) -> Result<HashSet<String>, sea_orm::DbErr> {
    let items: Vec<entity::items::Model> = Item::find()
        .filter(entity::items::Column::ChannelId.eq(channel.id))
        .all(connection)
        .await?;

    Ok(items
        .into_iter()
        .filter_map(|x| x.guid)
        .collect::<HashSet<String>>())
}

async fn get_users_of_channel(
    connection: &DatabaseConnection,
    channel_id: i32,
) -> Result<Vec<i32>, sea_orm::DbErr> {
    ChannelUser::find()
        .select_only()
        .column(entity::channel_users::Column::UserId)
        .filter(entity::channel_users::Column::ChannelId.eq(channel_id))
        .into_values::<_, QueryAs>()
        .all(connection)
        .await
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
