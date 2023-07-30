use sqlx::{Postgres, QueryBuilder, Result};

use crate::model::{NewItem, PagedResult, UserItem};
use crate::Pool;

/// Return a page of items of a given channel for a given user.
#[tracing::instrument(skip(db))]
pub async fn get_items_of_user<C>(
    db: &Pool,
    channel_id: Option<i32>,
    read: Option<bool>,
    starred: Option<bool>,
    user_id: i32,
    page_number: u64,
    page_size: u64,
) -> Result<PagedResult<UserItem>> {
    let mut base_query: QueryBuilder<Postgres> = QueryBuilder::new(
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
               channels.name       AS channel_name,
               channels.id         AS channel_id
        FROM items
                 RIGHT JOIN users_items ON items.id = users_items.item_id
                 RIGHT JOIN channels ON items.channel_id = channels.id
        WHERE users_items.user_id =
    "#,
    );
    base_query.push_bind(user_id);

    if let Some(channel_id) = channel_id {
        base_query.push(" AND users_items.channel_id = ");
        base_query.push_bind(channel_id);
    }

    if let Some(read) = read {
        base_query.push(" AND users_items.read = ");
        base_query.push_bind(read);
    }

    if let Some(starred) = starred {
        base_query.push(" AND users_items.starred = ");
        base_query.push_bind(starred);
    }

    base_query.push(
        r#"
        ORDER BY items.publish_timestamp DESC
        "#,
    );

    let base_query = base_query.sql();

    let mut page_query: QueryBuilder<Postgres> = QueryBuilder::new(base_query);
    page_query.push(" LIMIT ");
    page_query.push_bind(page_size as i64);

    page_query.push(" OFFSET ");
    page_query.push_bind((page_number as i64 - 1) * page_size as i64);

    let page_query = page_query.sql();

    let mut count_query: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT COUNT(*) AS num_items FROM ( 
        "#,
    );
    count_query.push(base_query);
    count_query.push(" ) AS sub_query ");

    let count_query = count_query.sql();

    let content = sqlx::query_as(page_query).fetch_all(db).await?;
    let total_items = sqlx::query_scalar(count_query)
        .fetch_optional(db)
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

/// Get all the item's GUID of a given channel.
#[tracing::instrument(skip(db))]
pub async fn get_all_items_guid_of_channel(
    db: &Pool,
    channel_id: i32,
) -> Result<Vec<Option<String>>> {
    sqlx::query_scalar!(
        r#"
        SELECT guid FROM items WHERE channel_id = $1
        "#,
        channel_id
    )
    .fetch_all(db)
    .await
}

/// Update the read status of an item for a given user
#[tracing::instrument(skip(db))]
pub async fn set_item_read<C>(db: &Pool, user_id: i32, ids: Vec<i32>, read: bool) -> Result<()> {
    for id in ids {
        sqlx::query!(
            r#"
                UPDATE users_items SET read = $1 WHERE user_id = $2 AND item_id = $3
            "#,
            read,
            user_id,
            id
        )
        .execute(db)
        .await?;
    }

    Ok(())
}

/// Update the starred status of an item for a given user
#[tracing::instrument(skip(db))]
pub async fn set_item_starred<C>(
    db: &Pool,
    user_id: i32,
    ids: Vec<i32>,
    starred: bool,
) -> Result<()> {
    for id in ids {
        sqlx::query!(
            r#"
                UPDATE users_items SET starred = $1 WHERE user_id = $2 AND item_id = $3
            "#,
            starred,
            user_id,
            id
        )
        .execute(db)
        .await?;
    }

    Ok(())
}

/// Insert an item in the database and associate it to all given users
#[tracing::instrument(skip(db))]
pub async fn insert_item_for_user(db: &Pool, item: &NewItem, user_ids: &[i32]) -> Result<()> {
    let item_id = insert_item(db, item).await?;

    for user_id in user_ids {
        insert_item_user(db, item_id, item.channel_id, user_id).await?;
    }

    Ok(())
}

/// Insert an item in the database
#[tracing::instrument(skip(db))]
async fn insert_item(db: &Pool, item: &NewItem) -> Result<i32> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO items (guid, title, url, content, fetch_timestamp, publish_timestamp, channel_id) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id
        "#,
        item.guid, item.title, item.url, item.content, item.fetch_timestamp, item.publish_timestamp, item.channel_id)
        .fetch_one(db).await
}

/// Insert an item in the database
#[tracing::instrument(skip(db))]
async fn insert_item_user(db: &Pool, item_id: i32, channel_id: i32, user_id: &i32) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO users_items (user_id, item_id, channel_id, read, starred) VALUES ($1, $2, $3, false, false)
        "#,
        user_id, item_id, channel_id)
        .execute(db).await?;

    Ok(())
}
