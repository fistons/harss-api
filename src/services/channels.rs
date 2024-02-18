use chrono::{DateTime, Utc};
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use sqlx::Result;
use tokio::task;
use tracing::error;
use tracing::{debug, info, instrument};

use crate::common::model::Channel;
use crate::common::model::ChannelError;
use crate::common::model::PagedResult;
use crate::common::model::UsersChannel;
use crate::common::rss::check_feed;

use super::fetching;
pub struct ChannelService {
    db: PgPool,
    redis: RedisPool,
}

impl ChannelService {
    pub fn new(db: PgPool, redis: RedisPool) -> Self {
        ChannelService { db, redis }
    }

    /// Returns the whole list of errors associated to the given channel id.
    #[instrument(skip(self))]
    pub async fn select_errors_by_chan_id(
        &self,
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
        .fetch_all(&self.db)
        .await?;

        Ok(result)
    }

    /// Returns an optional given channel with the given user's metadata.
    #[instrument(skip(self))]
    pub async fn select_by_id_and_user_id(
        &self,
        channel_id: i32,
        user_id: i32,
    ) -> Result<Option<UsersChannel>> {
        let result = sqlx::query_as!(
            UsersChannel,
            r#"
            SELECT      "channels"."id",
                        "channel_users"."name",
                        "channel_users"."notes",
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
            GROUP BY    "channels"."id", "channel_users"."name", "channel_users"."notes"
            "#,
            channel_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(result)
    }

    /// Mark the given channel as read for the given user
    #[instrument(skip(self))]
    pub async fn mark_channel_as_read(&self, channel_id: i32, user_id: i32) -> Result<()> {
        self.mark_channel(channel_id, user_id, true).await
    }

    /// Mark the given channel as unread for the given user
    #[instrument(skip(self))]
    pub async fn mark_channel_as_unread(&self, channel_id: i32, user_id: i32) -> Result<()> {
        self.mark_channel(channel_id, user_id, false).await
    }

    ///  Select all the channels of a user, along side the total number of items
    #[instrument(skip(self))]
    pub async fn select_page_by_user_id(
        &self,
        user_id: i32,
        page_number: u64,
        page_size: u64,
    ) -> Result<PagedResult<UsersChannel>> {
        let content = sqlx::query_as!(
        UsersChannel,
        r#"
        SELECT "channels"."id",
                "channel_users"."name",
                "channel_users"."notes",
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
        GROUP BY "channels"."id", "channel_users"."registration_timestamp", "channel_users"."name", "channel_users"."notes"
        ORDER BY "channel_users"."registration_timestamp" DESC
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        page_size as i64,
        (page_number as i64 - 1) * page_size as i64
    )
    .fetch_all(&self.db)
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
        .fetch_one(&self.db)
        .await?
        .unwrap_or(0) as u64;

        Ok(PagedResult::new(
            content,
            total_items,
            page_size,
            page_number,
        ))
    }

    /// Create or linked an existing channel to a user, returning the channel id
    #[instrument(skip(self))]
    pub async fn create_or_link_channel(
        &self,
        url: &str,
        name: Option<String>,
        notes: Option<String>,
        user_id: i32,
    ) -> Result<i32> {
        // Retrieve or create the channel
        let (channel_id, channel_name) = match sqlx::query!(
            r#"
            SELECT id, name FROM channels WHERE url = $1
            "#,
            url
        )
        .fetch_optional(&self.db)
        .await?
        {
            Some(result) => (result.id, result.name),
            None => self.create_new_channel(url).await?,
        };

        // Insert the channel in the users registered channel
        sqlx::query!(
            r#"
            INSERT INTO channel_users (channel_id, user_id, name, registration_timestamp, notes) 
            VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING
            "#,
            channel_id,
            user_id,
            name.unwrap_or(channel_name),
            Utc::now().into(),
            notes
        )
        .execute(&self.db)
        .await?;

        // Link all the existing items of the channel to the user
        sqlx::query_scalar!(
            r#"
            INSERT INTO users_items (user_id, item_id, channel_id, read, starred)
            SELECT $2, id, $1, false, false
            from items
            where channel_id = $1
            ON CONFLICT DO NOTHING
            "#,
            channel_id,
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(channel_id)
    }

    /// Enable a channel and reset it's failure count
    #[instrument(skip(self))]
    pub async fn enable_channel(&self, channel_id: i32) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE channels SET disabled = false, failure_count = 0 WHERE channels.id = $1
            "#,
            channel_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Disable a channel
    #[instrument(skip(self))]
    pub async fn disable_channel(&self, channel_id: i32) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE channels SET disabled = true WHERE channels.id = $1
            "#,
            channel_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Disable channels whom failure count is higher than the given threshold
    #[instrument(skip(self))]
    pub async fn disable_channels(&self, threshold: u32) -> Result<()> {
        let disabled_channels = sqlx::query!(
            r#"
            UPDATE channels SET disabled = true WHERE disabled = false AND failure_count >= $1
            "#,
            threshold as i32
        )
        .execute(&self.db)
        .await?;

        tracing::debug!("Disabled {} channels", disabled_channels.rows_affected());

        Ok(())
    }

    /// Return the list of user IDs of of a given channel
    #[instrument(skip(self))]
    pub async fn get_user_ids_of_channel(&self, channel_id: i32) -> Result<Vec<i32>> {
        sqlx::query_scalar!(
            r#"
            SELECT user_id FROM channel_users WHERE channel_id = $1
            "#,
            channel_id
        )
        .fetch_all(&self.db)
        .await
    }

    /// Return the list of all enabled channels
    #[instrument(skip(self))]
    pub async fn get_all_enabled_channels(&self) -> Result<Vec<Channel>> {
        sqlx::query_as!(
            Channel,
            r#"
            SELECT * FROM channels
            "#
        )
        .fetch_all(&self.db)
        .await
    }

    /// Update the last fetched timestamp of a channel
    #[instrument(skip(self))]
    pub async fn update_last_fetched(&self, channel_id: i32, date: &DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE channels SET last_update = $2 WHERE id = $1
            "#,
            channel_id,
            date.into()
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Retrieve the last update of channel
    #[instrument(skip(self))]
    pub async fn get_last_update(&self, channel_id: &i32) -> Result<Option<DateTime<Utc>>> {
        let last_update = sqlx::query!(
            r#"
            SELECT last_update FROM channels WHERE id = $1
            "#,
            channel_id
        )
        .fetch_one(&self.db)
        .await?;

        Ok(last_update.last_update)
    }

    /// Update the failure count of the given channel and insert the error in the dedicated table
    #[instrument(skip(self))]
    pub async fn fail_channel(&self, channel_id: i32, error_cause: &str) -> Result<()> {
        let mut transaction = self.db.begin().await?;
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
    #[instrument(skip(self))]
    async fn create_new_channel(&self, channel_url: &str) -> Result<(i32, String)> {
        let feed = check_feed(channel_url)
            .await
            .map_err(|_| sqlx::Error::RowNotFound)?; //TODO: Bad error type

        let channel = sqlx::query_as!(
            Channel,
            r#"
            INSERT INTO channels (name, url) VALUES ($1, $2) RETURNING *
            "#,
            feed.title.map(|x| x.content).unwrap_or(channel_url.into()),
            channel_url
        )
        .fetch_one(&self.db)
        .await?;

        let channel_id = channel.id;
        let channel_name = channel.name.clone();

        // Launch a fetch in a task
        let channel = channel.clone();
        let redis = self.redis.clone();
        let db = self.db.clone();
        task::spawn(async move {
            if let Err(err) = fetching::update_channel(&db, &redis, &channel).await {
                error!("Could not update channel {}: {:?}", channel.name, err);
            } else {
                debug!("Channel {} updated", channel.id);
            }
        });

        Ok((channel_id, channel_name))
    }

    async fn mark_channel(&self, channel_id: i32, user_id: i32, read: bool) -> Result<()> {
        sqlx::query!(
        r#"
        UPDATE users_items SET read = $3 WHERE users_items.channel_id = $1 AND users_items.user_id = $2
        "#,
        channel_id,
        user_id,
        read
    )
    .execute(&self.db)
    .await?;

        Ok(())
    }

    /// Unsubscribe a user from a channel
    #[instrument(skip(self))]
    pub async fn unsubscribe_channel(&self, channel_id: i32, user_id: i32) -> Result<()> {
        let mut transaction = self.db.begin().await?;

        let result = sqlx::query!(
            r#"
           DELETE FROM channel_users WHERE channel_id = $1 and user_id = $2
        "#,
            channel_id,
            user_id
        )
        .execute(&mut *transaction)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        debug!("User {} unsubscribed fron channel {}", user_id, channel_id);

        // If no user remains subscribed, delete the whole chan
        let result = sqlx::query!(
        r#"
           DELETE FROM channels WHERE id = $1 AND (SELECT count(*) FROM channel_users WHERE channel_id = $1) = 0
        "#,
        channel_id
        )
        .execute(&mut *transaction)
        .await?;

        if result.rows_affected() == 1 {
            info!("Deleted channel id {} from database", channel_id);
        }

        transaction.commit().await?;

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use crate::common::init_redis_connection;
    use speculoos::prelude::*;

    use super::*;

    fn init(db: PgPool) -> ChannelService {
        let redis = init_redis_connection();

        ChannelService { db, redis }
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn test_no_conflict_on_existing_channel_insertion(pool: PgPool) -> Result<()> {
        let service = init(pool);

        let channel_id = service
            .create_or_link_channel("https://www.canardpc.com/feed", None, None, 1) // 1 is root
            .await
            .unwrap();

        assert_that!(channel_id).is_equal_to(1);

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn test_user_get_items_on_registration(pool: PgPool) -> Result<()> {
        let service = init(pool);
        let _channel_id = service
            .create_or_link_channel("https://www.canardpc.com/feed", None, None, 2) // 2 is john_doe
            .await
            .unwrap();

        // assert_that!(channel_id).is_equal_to(1);
        // let items = service
        //     .get_items_of_user(Some(1), None, None, 2, 1, 400)
        //     .await
        //     .unwrap();
        // asserting!("John doe now has the 60 items of channel 1")
        //     .that(items.content())
        //     .has_length(60);

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn test_user_registration_on_empty_channel(pool: PgPool) -> Result<()> {
        let service = init(pool);
        let _channel_id = service
            .create_or_link_channel(
                "https://rss.slashdot.org/Slashdot/slashdotMain",
                None,
                None,
                1,
            ) // 1 is root
            .await
            .unwrap();

        // assert_that!(channel_id).is_equal_to(3);
        // let items = get_items_of_user(&pool, Some(3), None, None, 1, 1, 400) // Channel 3 is empty
        //     .await
        //     .unwrap();
        // asserting!("List of items is empty")
        //     .that(items.content())
        //     .is_empty();

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn test_channel_unsubscribe(pool: PgPool) -> Result<()> {
        let service = init(pool.clone());

        // Register the same channel for two users
        let channel_id_u1 = service
            .create_or_link_channel("https://www.canardpc.com/feed", None, None, 1)
            .await
            .unwrap();

        let channel_id_u2 = service
            .create_or_link_channel("https://www.canardpc.com/feed", None, None, 2)
            .await
            .unwrap();
        assert_eq!(channel_id_u1, channel_id_u2);

        // Unsubscribe user 1 from channel and check.
        service.unsubscribe_channel(channel_id_u1, 1).await.unwrap();
        let result = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM channel_users WHERE channel_id = $1 AND user_id = $2",
            channel_id_u1,
            1,
        )
        .fetch_one(&pool)
        .await;
        assert_eq!(Some(0i64), result.unwrap());

        // Unsubscribe user 2 from channel and check.
        service.unsubscribe_channel(channel_id_u1, 2).await.unwrap();
        let result = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM channel_users WHERE channel_id = $1 AND user_id = $2",
            channel_id_u1,
            1,
        )
        .fetch_one(&pool)
        .await;
        assert_eq!(Some(0i64), result.unwrap());

        // Check that the channel have been completely removed
        let result = sqlx::query_scalar!(
            "SELECT count(id) as count FROM channels WHERE id = $1",
            channel_id_u1
        )
        .fetch_one(&pool)
        .await;
        assert_eq!(Some(0i64), result.unwrap());

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn test_add_notes_and_custom_name(pool: PgPool) -> Result<()> {
        let service = init(pool);
        let channel_id = service
            .create_or_link_channel(
                "https://www.canardpc.com/feed",
                Some("My custom name".to_owned()),
                Some("My custom notes".to_owned()),
                2,
            )
            .await
            .unwrap();

        let channel = service
            .select_by_id_and_user_id(channel_id, 2)
            .await
            .unwrap()
            .unwrap();

        assert_eq!("My custom name", channel.name);
        assert_that!(channel.notes).is_equal_to(Some("My custom notes".to_owned()));

        let channel_from_other_user = service
            .select_by_id_and_user_id(channel_id, 1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!("Canard PC", channel_from_other_user.name);
        assert_that!(channel_from_other_user.notes).is_equal_to(None);

        Ok(())
    }
}
