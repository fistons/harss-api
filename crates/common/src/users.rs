use secrecy::Secret;
use sqlx::Result;

use crate::model::{PagedResult, User, UserRole};
use crate::password::encode_password;
use crate::Pool;

/// Return the user matching the username
#[tracing::instrument(skip(db))]
pub async fn get_user_by_username(db: &Pool, wanted_username: &str) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole" FROM users WHERE username = $1
        "#,
        wanted_username
    )
    .fetch_optional(db)
    .await
}

/// Return the user matching the id
#[tracing::instrument(skip(db), level = "debug")]
pub async fn get_user_by_id(db: &Pool, id: i32) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole" FROM users WHERE id = $1
        "#,
        id
    )
    .fetch_optional(db)
    .await
}

/// List all the users
#[tracing::instrument(skip(db))]
pub async fn list_users(db: &Pool, page_number: u64, page_size: u64) -> Result<PagedResult<User>> {
    let content = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole" FROM users
        ORDER BY id
        LIMIT $1 OFFSET $2
        "#,
        page_size as i64,
        (page_number as i64 - 1) * page_size as i64
    )
    .fetch_all(db)
    .await?;

    let total_items = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM users
        "#,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0) as u64;

    Ok(PagedResult::new(
        content,
        total_items,
        page_size,
        page_number,
    ))
}

/// Create a new user
#[tracing::instrument(skip(db))]
pub async fn create_user(
    db: &Pool,
    login: &str,
    password: &Secret<String>,
    user_role: UserRole,
) -> Result<User> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (username, password, role) VALUES ($1, $2, $3) 
        RETURNING id, username, password, role as "role: UserRole"
        "#,
        login,
        encode_password(password),
        user_role as UserRole
    )
    .fetch_one(db)
    .await
}

/// Update a user's password
#[tracing::instrument(skip(db))]
pub async fn update_user_password(
    db: &Pool,
    user_id: i32,
    new_password: &Secret<String>,
) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE users SET password = $1 WHERE id=$2
    "#,
        encode_password(new_password),
        user_id
    )
    .execute(db)
    .await?;

    //TODO: Must return a dedicated error
    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }

    Ok(())
}
