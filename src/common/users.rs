use redis::{AsyncCommands, SetExpiry, SetOptions};
use secrecy::{ExposeSecret, Secret};
use sqlx::Result;

use crate::common::model::{PagedResult, User, UserRole};
use crate::common::password::{encode_password, hash_email};
use crate::common::Pool;

use deadpool_redis::Pool as RedisPool;

use super::email::send_reset_password_email;

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

/// Return the user matching the id
#[tracing::instrument(skip(db), level = "debug")]
pub async fn get_user_by_email(db: &Pool, email: &Secret<String>) -> Result<Option<User>> {
    let encoded_email = hash_email(&Some(email.clone()));
    tracing::info!("{:?} {encoded_email:?}", email.expose_secret());
    sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole" FROM users WHERE email = $1
        "#,
        encoded_email
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
    email: &Option<Secret<String>>,
    user_role: UserRole,
) -> Result<User> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (username, password, email, role) VALUES ($1, $2, $3, $4) 
        RETURNING id, username, password, role as "role: UserRole"
        "#,
        login,
        encode_password(password),
        hash_email(email),
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

pub async fn reset_password(
    db: &Pool,
    redis: &RedisPool,
    token: &Secret<String>,
    new_password: &Secret<String>,
    username: &str,
) -> Result<()> {
    let token = token.expose_secret();

    let user = get_user_by_username(db, username).await?;
    if let Some(user) = user {
        let key = format!("user.reset-token.{}", user.id);
        let registered_token: Option<String> = redis.get().await.unwrap().get(&key).await.unwrap();
        if let Some(registered_token) = registered_token {
            if registered_token == *token {
                update_user_password(db, user.id, new_password).await?;

                redis
                    .get()
                    .await
                    .unwrap()
                    .del::<String, String>(key)
                    .await
                    .unwrap();

                return Ok(());
            }
        }
    }

    Err(sqlx::Error::RowNotFound)
}

#[tracing::instrument(skip(db, redis))]
pub async fn reset_password_request(
    db: &Pool,
    redis: &RedisPool,
    email: &Secret<String>,
) -> Result<()> {
    let user = get_user_by_email(db, email).await?;

    if let Some(user) = user {
        let reset_token = uuid::Uuid::new_v4();

        let key = format!("user.reset-token.{}", user.id);
        let options = SetOptions::default().with_expiration(SetExpiry::EX(60 * 15));
        let _ = redis
            .get()
            .await
            .unwrap()
            .set_options::<&str, &str, String>(&key, &reset_token.to_string(), options)
            .await;

        if let Err(e) = send_reset_password_email(
            email.expose_secret(),
            &user.username,
            &reset_token.to_string(),
        )
        .await
        {
            tracing::error!("Could not send email {}", e);
        }
    } else {
        tracing::debug!("Email not found for reset password request");
    };

    Ok(())
}
