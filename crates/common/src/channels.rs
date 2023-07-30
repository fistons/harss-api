use chrono::{DateTime, Utc};
use sqlx::Result;

use crate::model::{Channel, ChannelError, PagedResult, UsersChannel};
use crate::rss::check_feed;
use crate::{DbError, Pool};

/// Returns the whole list of errors associated to the given channel id.
#[tracing::instrument(skip(db))]
pub async fn select_errors_by_chan_id(
    db: &Pool,
    channel_id: i32,
    user_id: i32,
) -> Result<Vec<ChannelError>> {
    let result = sqlx::query_as!(
        ChannelError,
        r#"
        SELECT  channels_errors.id,
                channels_errors.channel_id,
                channels_errors.error_timestamp,
                channels_errors.error_reason,
                channels.name AS channel_name
        FROM    channels_errors
                JOIN channels ON channels_errors.channel_id = channels.id
                JOIN channel_users ON channels_errors.channel_id = channel_users.channel_id
        WHERE   channels_errors.channel_id = $1
        AND     channel_users.user_id = $2
        "#,
        channel_id,
        user_id
    )
    .fetch_all(db)
    .await?;

    Ok(result)
}

/// Returns an optional given channel with the given user's metadata.
#[tracing::instrument(skip(db))]
pub async fn select_by_id_and_user_id(
    db: &Pool,
    channel_id: i32,
    user_id: i32,
) -> Result<Option<UsersChannel>> {
    let result = sqlx::query_as!(
        UsersChannel,
        r#"
        SELECT      "channels"."id",
                    "channels"."name",
                    "channels"."url",
                    "channels"."registration_timestamp",
                    "channels"."last_update",
                    "channels"."disabled",
                    "channels"."failure_count",
                    COUNT("users_items"."item_id") AS "items_count",
                    SUM(CAST("read" AS integer))   AS "items_read"
        FROM        "channels"
                    RIGHT JOIN "channel_users" ON "channels"."id" = "channel_users"."channel_id"
                    LEFT JOIN "users_items" ON "channels"."id" = "users_items"."channel_id"
        WHERE       "channel_users"."user_id" = $2
        AND         "channel_users"."channel_id" = $1
        GROUP BY    "channels"."id"
        "#,
        channel_id,
        user_id
    )
    .fetch_optional(db)
    .await?;

    Ok(result)
}

/// Mark the given channel as read for the given user
#[tracing::instrument(skip(db))]
pub async fn mark_channel_as_read(db: &Pool, channel_id: i32, user_id: i32) -> Result<()> {
    mark_channel(db, channel_id, user_id, true).await
}

/// Mark the given channel as unread for the given user
#[tracing::instrument(skip(db))]
pub async fn mark_channel_as_unread(db: &Pool, channel_id: i32, user_id: i32) -> Result<()> {
    mark_channel(db, channel_id, user_id, false).await
}

///  Select all the channels of a user, along side the total number of items
#[tracing::instrument(skip(db))]
pub async fn select_page_by_user_id(
    db: &Pool,
    user_id: i32,
    page_number: u64,
    page_size: u64,
) -> Result<PagedResult<UsersChannel>> {
    let content = sqlx::query_as!(
        UsersChannel,
        r#"
        SELECT "channels"."id",
                "channels"."name",
                "channels"."url",
                "channels"."registration_timestamp",
                "channels"."last_update",
                "channels"."disabled",
                "channels"."failure_count",
                COUNT("users_items"."item_id") AS "items_count",
                SUM(CAST("read" AS integer))   AS "items_read"
        FROM "channels"
                 RIGHT JOIN "channel_users" ON "channels"."id" = "channel_users"."channel_id"
                 LEFT JOIN "users_items" ON "channels"."id" = "users_items"."channel_id"
        WHERE "channel_users"."user_id" = $1
        GROUP BY "channels"."id", "channel_users"."registration_timestamp"
        ORDER BY "channel_users"."registration_timestamp" DESC
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        page_size as i64,
        (page_number as i64 - 1) * page_size as i64
    )
    .fetch_all(db)
    .await?;

    let total_items = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS num_items
            FROM (SELECT "channels"."id",
             "channels"."name",
             "channels"."url",
             "channels"."registration_timestamp",
             "channels"."last_update",
             "channels"."disabled",
             "channels"."failure_count",
             COUNT("users_items"."item_id") AS "items_count",
             SUM(CAST("read" AS integer))   AS "items_read"
        FROM "channels"
             RIGHT JOIN "channel_users" ON "channels"."id" = "channel_users"."channel_id"
             LEFT JOIN "users_items" ON "channels"."id" = "users_items"."channel_id"
        WHERE "channel_users"."user_id" = $1
        GROUP BY "channels"."id", "channel_users"."registration_timestamp"
        ORDER BY "channel_users"."registration_timestamp" DESC) AS "sub_query"
        "#,
        user_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0) as u64;

    let total_pages = total_items / page_size;
    let elements_number = content.len();

    Ok(PagedResult {
        content,
        page_number,
        page_size,
        total_pages,
        elements_number,
        total_items,
    })
}

/// Create or linked an existing channel to a user, returning the channel id
#[tracing::instrument(skip(db))]
pub async fn create_or_link_channel(db: &Pool, url: &str, user_id: i32) -> Result<i32> {
    // Retrieve or create the channel
    let channel_id = match sqlx::query_scalar!(
        r#"
        SELECT id FROM channels WHERE url = $1
        "#,
        url
    )
    .fetch_optional(db)
    .await?
    {
        Some(id) => id,
        None => create_new_channel(db, url).await?,
    };

    // Insert the channel in the users registered channel
    sqlx::query!(
        r#"
        INSERT INTO channel_users (channel_id, user_id, registration_timestamp) 
        VALUES ($1, $2, $3)
        "#,
        channel_id,
        user_id,
        Utc::now().into()
    )
    .execute(db)
    .await?;

    //TODO: Copy all the items to the new user

    Ok(channel_id)
}

/// Enable a channel and reset it's failure count
#[tracing::instrument(skip(db))]
pub async fn enable_channel(db: &Pool, channel_id: i32) -> Result<()> {
    sqlx::query!(
        r#"
         UPDATE channels SET disabled = false, failure_count = 0 WHERE channels.id = $1
        "#,
        channel_id
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Disable a channel
pub async fn disable_channel(db: &Pool, channel_id: i32) -> Result<()> {
    sqlx::query!(
        r#"
         UPDATE channels SET disabled = true WHERE channels.id = $1
        "#,
        channel_id
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Disable channels whom failure count is higher than the given threshold
#[tracing::instrument(skip(db))]
pub async fn disable_channels(db: &Pool, threshold: u32) -> Result<()> {
    let disabled_channels = sqlx::query!(
        r#"
        UPDATE channels SET disabled = true WHERE disabled = false AND failure_count >= $1
        "#,
        threshold as i32
    )
    .execute(db)
    .await?;

    tracing::debug!("Disabled {} channels", disabled_channels.rows_affected());

    Ok(())
}

/// Return the list of user IDs of of a given channel
#[tracing::instrument(skip(db))]
pub async fn get_user_ids_of_channel(db: &Pool, channel_id: i32) -> Result<Vec<i32>> {
    sqlx::query_scalar!(
        r#"
        SELECT user_id FROM channel_users WHERE channel_id = $1
        "#,
        channel_id
    )
    .fetch_all(db)
    .await
}

/// Return the list of all enabled channels
#[tracing::instrument(skip(db))]
pub async fn get_all_enabled_channels(db: &Pool) -> Result<Vec<Channel>> {
    sqlx::query_as!(
        Channel,
        r#"
        SELECT * FROM channels
        "#
    )
    .fetch_all(db)
    .await
}

/// Update the last fetched timestamp of a channel
#[tracing::instrument(skip(db))]
pub async fn update_last_fetched(db: &Pool, channel_id: i32, date: DateTime<Utc>) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE channels SET last_update = $2 WHERE id = $1
        "#,
        channel_id,
        date.into()
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Update the failure count of the given channel and insert the error in the dedicated table
/// TODO: Transaction
#[tracing::instrument(skip(db))]
pub async fn fail_channel(db: &Pool, channel_id: i32, error_cause: &str) -> Result<()> {
    let mut transaction = db.begin().await?;
    sqlx::query!(
        r#"
        UPDATE channels SET failure_count = failure_count + 1 WHERE id = $1
        "#,
        channel_id
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query!(
        r#"
       INSERT INTO channels_errors (channel_id, error_timestamp, error_reason) VALUES ($1, $2, $3)
        "#,
        channel_id,
        Utc::now().into(),
        error_cause
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

/// # Create a new channel in the database, returning the created channel id
#[tracing::instrument(skip(db))]
async fn create_new_channel(db: &Pool, channel_url: &str) -> Result<i32> {
    let feed = check_feed(channel_url)
        .await
        .map_err(|_| DbError::RowNotFound)?; //TODO: Bad error type

    let result = sqlx::query!(
        r#"
        INSERT INTO channels (name, url) VALUES ($1, $2) RETURNING id
        "#,
        feed.title.map(|x| x.content).unwrap_or(channel_url.into()),
        channel_url
    )
    .fetch_one(db)
    .await?;

    Ok(result.id)
}

async fn mark_channel(db: &Pool, channel_id: i32, user_id: i32, read: bool) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE users_items SET read = $3 WHERE users_items.channel_id = $1 AND users_items.user_id = $2
        "#,
        channel_id,
        user_id,
        read
    )
    .execute(db)
    .await?;

    Ok(())
}
