use crate::common::model::{NewItem, PagedResult, UserItem};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder, Result};

pub struct ItemService {
    db: PgPool,
}

impl ItemService {
    pub fn new(db: PgPool) -> Self {
        ItemService { db }
    }

    /// Return a page of items of a given channel for a given user.
    #[tracing::instrument(skip(self))]
    pub async fn get_items_of_user(
        &self,
        channel_id: Option<i32>,
        read: Option<bool>,
        starred: Option<bool>,
        user_id: i32,
        page_number: u64,
        page_size: u64,
    ) -> Result<PagedResult<UserItem>> {
        let base_part = r#"
        SELECT items.id,
               items.guid,
               items.title,
               items.url,
               items.content,
               items.fetch_timestamp,
               items.publish_timestamp,
               users_items.read    AS read,
               users_items.starred AS starred,
               users_items.notes   AS notes,
               channel_users.name       AS channel_name,
               items.channel_id AS channel_id
        FROM items
                 RIGHT JOIN users_items ON items.id = users_items.item_id
                 RIGHT JOIN channel_users ON items.channel_id = channel_users.channel_id and users_items.user_id = channel_users.user_id
        WHERE users_items.user_id =
    "#;

        let mut page_query: QueryBuilder<Postgres> = QueryBuilder::new(base_part);
        page_query.push_bind(user_id);

        add_filters(&mut page_query, channel_id, read, starred);

        page_query.push(
            r#"
        ORDER BY items.publish_timestamp DESC
        "#,
        );

        page_query.push(" LIMIT ");
        page_query.push_bind(page_size as i64);

        page_query.push(" OFFSET ");
        page_query.push_bind((page_number as i64 - 1) * page_size as i64);

        let mut count_query: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"
        SELECT COUNT(*) AS num_items FROM (
        "#,
        );
        count_query.push(base_part);
        count_query.push_bind(user_id);
        add_filters(&mut count_query, channel_id, read, starred);
        count_query.push(" ) AS sub_query ");

        let content = page_query.build_query_as().fetch_all(&self.db).await?;
        let total_items = count_query
            .build_query_scalar()
            .fetch_optional(&self.db)
            .await?
            .unwrap_or(0i64) as u64;

        Ok(PagedResult::new(
            content,
            total_items,
            page_size,
            page_number,
        ))
    }

    /// Get all the item's GUID of a given channel.
    #[tracing::instrument(skip(self))]
    pub async fn get_all_items_guid_of_channel(
        &self,
        channel_id: i32,
    ) -> Result<Vec<Option<String>>> {
        sqlx::query_scalar!(
            r#"
        SELECT guid FROM items WHERE channel_id = $1
        "#,
            channel_id
        )
        .fetch_all(&self.db)
        .await
    }

    /// Update the read status of an item for a given user
    #[tracing::instrument(skip(self))]
    pub async fn set_item_read(&self, user_id: i32, ids: Vec<i32>, read: bool) -> Result<()> {
        for id in ids {
            sqlx::query!(
                r#"
                UPDATE users_items SET read = $1 WHERE user_id = $2 AND item_id = $3
            "#,
                read,
                user_id,
                id
            )
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    /// Update the starred status of an item for a given user
    #[tracing::instrument(skip(self))]
    pub async fn set_item_starred(&self, user_id: i32, ids: Vec<i32>, starred: bool) -> Result<()> {
        //TODO: transactional
        for id in ids {
            sqlx::query!(
                r#"
                UPDATE users_items SET starred = $1 WHERE user_id = $2 AND item_id = $3
            "#,
                starred,
                user_id,
                id
            )
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    /// Insert an item in the database and associate it to all given users
    #[tracing::instrument(skip(self))]
    pub async fn insert_items_delta_for_all_registered_users(
        &self,
        channel_id: i32,
        fetch_timestamp: &DateTime<Utc>,
    ) -> Result<()> {
        //TODO: transactional

        let user_ids = self.get_user_ids_of_channel(channel_id).await?;

        for user_id in user_ids {
            self.insert_item_user(&channel_id, &user_id, fetch_timestamp)
                .await?;
        }

        Ok(())
    }

    /// Insert items in the database
    #[tracing::instrument(skip(self))]
    pub async fn insert_items(&self, items: &Vec<NewItem>) -> Result<Vec<i32>> {
        let mut guids: Vec<Option<String>> = vec![];
        let mut titles: Vec<Option<String>> = vec![];
        let mut urls: Vec<Option<String>> = vec![];
        let mut contents: Vec<Option<String>> = vec![];
        let mut fetch_timestamps: Vec<DateTime<Utc>> = vec![];
        let mut publish_timestamps: Vec<Option<DateTime<Utc>>> = vec![];
        let mut channel_ids: Vec<i32> = vec![];

        for item in items {
            guids.push(item.guid.clone());
            titles.push(item.title.clone());
            urls.push(item.url.clone());
            contents.push(item.content.clone());
            fetch_timestamps.push(item.fetch_timestamp);
            publish_timestamps.push(item.publish_timestamp);
            channel_ids.push(item.channel_id);
        }

        // Postgres magic: https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-bind-an-array-to-a-values-clause-how-can-i-do-bulk-inserts
        // Also, sqlx magic: https://github.com/launchbadge/sqlx/issues/571#issuecomment-664910255
        sqlx::query_scalar!(
        r#"
        INSERT INTO items (guid, title, url, content, fetch_timestamp, publish_timestamp, channel_id)
        SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::timestamptz[], $6::timestamptz[], $7::int[])
        RETURNING id
        "#,
        &guids[..] as _, &titles[..] as _, &urls[..] as _, &contents[..] as _, &fetch_timestamps[..], &publish_timestamps[..] as _, &channel_ids[..])
        .fetch_all(&self.db).await
    }

    /// Add a note to a item for a user.
    /// The user_id is needed to insure that a user does not try to add a note on someone else item.
    pub async fn add_notes(&self, notes: String, user_id: i32, item_id: i32) -> Result<()> {
        let r = sqlx::query!(
            r#"
        UPDATE users_items SET notes = $1 WHERE item_id = $2 and user_id = $3
        "#,
            notes,
            item_id,
            user_id
        )
        .execute(&self.db)
        .await?;

        if r.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    /// Get a particular item for a given user
    /// The user_id is needed to insure that a user does not try to add a note on someone else item.
    #[tracing::instrument(skip(self))]
    pub async fn get_one_item(&self, item_id: i32, user_id: i32) -> Result<Option<UserItem>> {
        sqlx::query_as!(
            UserItem,
            r#"
        SELECT items.id,
               items.guid,
               items.title,
               items.url,
               items.content,
               items.fetch_timestamp,
               items.publish_timestamp,
               users_items.read    AS read,
               users_items.starred AS starred,
               users_items.notes    AS notes,
               channel_users.name       AS channel_name,
               channel_users.channel_id AS channel_id
        FROM items
               RIGHT JOIN users_items ON items.id = users_items.item_id
               RIGHT JOIN channel_users ON items.channel_id = channel_users.channel_id
        WHERE users_items.user_id = $1 AND users_items.item_id = $2
        "#,
            user_id,
            item_id
        )
        .fetch_optional(&self.db)
        .await
    }

    /// Insert the delta of the missing user's items for a given channel
    #[tracing::instrument(skip(self))]
    async fn insert_item_user(
        &self,
        channel_id: &i32,
        user_id: &i32,
        timestamp: &DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
        r#"
        INSERT INTO users_items (user_id, item_id, channel_id, read, starred, added_timestamp)
        SELECT  $1, id, $2, false, false, $3
        FROM    items
        WHERE   channel_id = $2
        AND     fetch_timestamp > COALESCE((SELECT MAX(added_timestamp) FROM users_items WHERE user_id = $1 AND channel_id = $2), TO_TIMESTAMP(0));
        "#,
        user_id, channel_id, timestamp)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn get_user_ids_of_channel(&self, channel_id: i32) -> Result<Vec<i32>> {
        sqlx::query_scalar!(
            r#"
        SELECT user_id FROM channel_users WHERE channel_id = $1
        "#,
            channel_id
        )
        .fetch_all(&self.db)
        .await
    }
}
fn add_filters(
    query: &mut QueryBuilder<Postgres>,
    channel_id: Option<i32>,
    read: Option<bool>,
    starred: Option<bool>,
) {
    if let Some(channel_id) = channel_id {
        query.push(" AND users_items.channel_id = ");
        query.push_bind(channel_id);
    }

    if let Some(read) = read {
        query.push(" AND users_items.read = ");
        query.push_bind(read);
    }

    if let Some(starred) = starred {
        query.push(" AND users_items.starred = ");
        query.push_bind(starred);
    }
}

#[cfg(test)]
mod tests {
    use speculoos::prelude::*;

    use super::*;

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn basic_without_filter(db: PgPool) -> Result<()> {
        let item_service = ItemService { db };

        let page = item_service
            .get_items_of_user(None, None, None, 1, 1, 20)
            .await?;

        assert_that!(page.page_size()).is_equal_to(&20);
        assert_that!(page.total_pages()).is_equal_to(&4);
        assert_that!(page.total_items()).is_equal_to(&78);
        assert_that!(page.elements_number()).is_equal_to(&20);
        assert_that!(page.page_number()).is_equal_to(&1);
        assert_that!(page.content()).has_length(20);

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn basic_channel_filter(db: PgPool) -> Result<()> {
        let item_service = ItemService { db };
        let page = item_service
            .get_items_of_user(Some(1), None, None, 1, 1, 20)
            .await?;

        assert_that!(page.page_size()).is_equal_to(&20);
        assert_that!(page.total_pages()).is_equal_to(&3);
        assert_that!(page.total_items()).is_equal_to(&60);
        assert_that!(page.elements_number()).is_equal_to(&20);
        assert_that!(page.page_number()).is_equal_to(&1);
        assert_that!(page.content()).has_length(20);

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn basic_read_filter(db: PgPool) -> Result<()> {
        let item_service = ItemService { db };
        let page = item_service
            .get_items_of_user(None, Some(true), None, 1, 1, 20)
            .await?;

        assert_that!(page.page_size()).is_equal_to(&20);
        assert_that!(page.total_pages()).is_equal_to(&4);
        assert_that!(page.total_items()).is_equal_to(&62);
        assert_that!(page.elements_number()).is_equal_to(&20);
        assert_that!(page.page_number()).is_equal_to(&1);
        assert_that!(page.content()).has_length(20);

        let page = item_service
            .get_items_of_user(None, Some(false), None, 1, 1, 20)
            .await?;

        assert_that!(page.page_size()).is_equal_to(&20);
        assert_that!(page.total_pages()).is_equal_to(&1);
        assert_that!(page.total_items()).is_equal_to(&16);
        assert_that!(page.elements_number()).is_equal_to(&16);
        assert_that!(page.page_number()).is_equal_to(&1);
        assert_that!(page.content()).has_length(16);

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn basic_starred_filter(db: PgPool) -> Result<()> {
        let item_service = ItemService { db };
        let page = item_service
            .get_items_of_user(None, None, Some(true), 1, 1, 20)
            .await?;

        assert_that!(page.page_size()).is_equal_to(&20);
        assert_that!(page.total_pages()).is_equal_to(&1);
        assert_that!(page.total_items()).is_equal_to(&3);
        assert_that!(page.elements_number()).is_equal_to(&3);
        assert_that!(page.page_number()).is_equal_to(&1);
        assert_that!(page.content()).has_length(3);

        let page = item_service
            .get_items_of_user(None, None, Some(false), 1, 1, 20)
            .await?;

        assert_that!(page.page_size()).is_equal_to(&20);
        assert_that!(page.total_pages()).is_equal_to(&4);
        assert_that!(page.total_items()).is_equal_to(&75);
        assert_that!(page.elements_number()).is_equal_to(&20);
        assert_that!(page.page_number()).is_equal_to(&1);
        assert_that!(page.content()).has_length(20);

        Ok(())
    }

    #[sqlx::test(fixtures("base_fixtures"), migrations = "./migrations")]
    async fn basic_all_filters(db: PgPool) -> Result<()> {
        let item_service = ItemService { db };
        let page = item_service
            .get_items_of_user(Some(1), Some(true), Some(true), 1, 1, 20)
            .await?;

        assert_that!(page.page_size()).is_equal_to(&20);
        assert_that!(page.total_pages()).is_equal_to(&1);
        assert_that!(page.total_items()).is_equal_to(&2);
        assert_that!(page.elements_number()).is_equal_to(&2);
        assert_that!(page.page_number()).is_equal_to(&1);
        assert_that!(page.content()).has_length(2);

        Ok(())
    }
}
