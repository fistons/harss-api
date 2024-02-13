use redis::{AsyncCommands, SetExpiry, SetOptions};
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;
use sqlx::Result;
use tracing::{debug, info};

use crate::common::model::{PagedResult, User, UserRole};
use crate::common::password::encode_password;
use crate::common::Pool;
use sha2::{Digest, Sha256};

use deadpool_redis::Pool as RedisPool;

use super::email::send_email;

/// Return the user matching the username
pub async fn get_user_by_username(db: &Pool, wanted_username: &str) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole", email_verified FROM users WHERE username = $1
        "#,
        wanted_username
    )
    .fetch_optional(db)
    .await
}

/// Return the user matching the id
pub async fn get_user_by_id(db: &Pool, id: i32) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole", email_verified FROM users WHERE id = $1
        "#,
        id
    )
    .fetch_optional(db)
    .await
}

/// Return the user matching the id
pub async fn get_user_by_email(db: &Pool, email: &Secret<String>) -> Result<Option<User>> {
    let encoded_email = hash_email(&Some(email.clone()));
    tracing::info!("{:?} {encoded_email:?}", email.expose_secret());
    sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole", email_verified FROM users WHERE email = $1 AND email_verified = true
        "#,
        encoded_email
    )
    .fetch_optional(db)
    .await
}

/// List all the users
pub async fn list_users(db: &Pool, page_number: u64, page_size: u64) -> Result<PagedResult<User>> {
    let content = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole", email_verified FROM users
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
pub async fn create_user(
    redis: &RedisPool,
    db: &Pool,
    login: &str,
    password: &Secret<String>,
    email: &Option<Secret<String>>,
    user_role: &UserRole,
) -> anyhow::Result<User> {
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (username, password, email, role, email_verified) VALUES ($1, $2, $3, $4, false) 
        RETURNING id, username, password, role as "role: UserRole", email_verified
        "#,
        login,
        encode_password(password),
        hash_email(email),
        user_role as &UserRole
    )
    .fetch_one(db)
    .await?; //TODO Make a beautifull error on unique constraint violation

    if let Some(email) = email {
        debug!(
            "User {} (id. {}) has provided an email during creation, sending confirmation",
            login, user.id
        );
        send_confirmation_email(redis, &user, email).await?;
    }

    Ok(user)
}

/// Update a user's password
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
) -> anyhow::Result<()> {
    let token = token.expose_secret();

    let user = get_user_by_username(db, username).await?;
    if let Some(user) = user {
        let key = format!("user.{}.reset-token", user.id);
        let registered_token: Option<String> = redis.get().await?.get(&key).await?;
        if let Some(registered_token) = registered_token {
            if registered_token == *token {
                update_user_password(db, user.id, new_password).await?;

                redis.get().await?.del::<String, usize>(key).await?;

                return Ok(());
            }
        }
    }

    Err(sqlx::Error::RowNotFound)?
}

pub async fn reset_password_request(
    db: &Pool,
    redis: &RedisPool,
    email: &Secret<String>,
) -> anyhow::Result<()> {
    let user = get_user_by_email(db, email).await?;

    if let Some(user) = user {
        let key = format!("user.{}.reset-token", user.id);
        let ttl_in_minutes = 15;
        let reset_token = generate_and_persist_token(redis, &key, ttl_in_minutes * 60).await?;

        let data = ResetPasswordData {
            dest_name: &user.username,
            dest_email: email.expose_secret(),
            token: &reset_token,
            token_ttl: ttl_in_minutes,
            user_id: user.id,
        };

        if let Err(e) = send_email("reset_password", &data).await {
            tracing::error!("Could not send email {}", e);
        }
    } else {
        tracing::debug!("Email not found for reset password request");
    };

    Ok(())
}

pub async fn update_user(
    db: &Pool,
    redis: &RedisPool,
    user_id: i32,
    email: &Option<Secret<String>>,
) -> anyhow::Result<()> {
    let user = get_user_by_id(db, user_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    if let Some(secret_email) = email {
        send_confirmation_email(redis, &user, secret_email).await?;

        sqlx::query!(
            r#"UPDATE users SET email = $1, email_verified = false WHERE id = $2"#,
            hash_email(email),
            user.id
        )
        .execute(db)
        .await?;
    }

    Ok(())
}

pub async fn confirm_email(
    db: &Pool,
    redis: &RedisPool,
    user_id: i32,
    token: &Secret<String>,
) -> anyhow::Result<()> {
    let key = format!("user.{}.confirm-email-token", user_id);
    let registered_token: Option<String> = redis.get().await?.get(&key).await?;

    if let Some(registered_token) = registered_token {
        if registered_token == *token.expose_secret() {
            sqlx::query!(
                "UPDATE users SET email_verified = true WHERE id = $1",
                user_id
            )
            .execute(db)
            .await?;

            redis.get().await?.del::<String, usize>(key).await?;

            return Ok(());
        }
    }
    Err(sqlx::Error::RowNotFound)?
}

pub async fn delete_user(db: &Pool, redis: &RedisPool, user_id: i32) -> anyhow::Result<()> {
    let result = sqlx::query!(r#"DELETE FROM users WHERE id = $1"#, user_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound)?;
    }

    delete_user_redis_keys(redis, user_id).await?;
    info!("Deleted user {}", user_id);
    Ok(())
}

async fn delete_user_redis_keys(redis: &RedisPool, user_id: i32) -> anyhow::Result<()> {
    debug!("Removing redis keys of user {}", user_id);
    for key in redis
        .get()
        .await?
        .keys::<String, Vec<String>>(format!("user.{}.*", user_id))
        .await?
        .iter()
    {
        debug!("Removing redis keys {} of user {}", key, user_id);
        redis.get().await?.del::<&str, usize>(key).await?;
    }

    Ok(())
}

async fn generate_and_persist_token(
    redis: &RedisPool,
    key: &str,
    ttl_in_seconds: usize,
) -> anyhow::Result<String> {
    let token = uuid::Uuid::new_v4().to_string();

    let options = SetOptions::default().with_expiration(SetExpiry::EX(ttl_in_seconds));
    let _ = redis
        .get()
        .await?
        .set_options::<&str, &str, String>(key, &token, options)
        .await;

    Ok(token)
}

/// Hash an email adresse using sha256
fn hash_email(email: &Option<Secret<String>>) -> Option<String> {
    if let Some(email) = email {
        let mut hasher = Sha256::new();

        hasher.update(email.expose_secret());
        let hash = hasher.finalize();

        return Some(String::from_utf8_lossy(&hash).to_string());
    }
    None
}

async fn send_confirmation_email(
    redis: &RedisPool,
    user: &User,
    email: &Secret<String>,
) -> anyhow::Result<()> {
    let key = format!("user.{}.confirm-email-token", user.id);
    let ttl_in_days = 15; //TODO variable please
    let reset_token = generate_and_persist_token(redis, &key, ttl_in_days * 86400).await?;
    let data = ResetPasswordData {
        dest_name: &user.username,
        dest_email: email.expose_secret(),
        token: &reset_token,
        token_ttl: ttl_in_days,
        user_id: user.id,
    };

    send_email("confirm_email", &data).await
}

#[derive(Serialize)]
struct ResetPasswordData<'a> {
    dest_name: &'a str,
    dest_email: &'a str,
    token: &'a str,
    token_ttl: usize,
    user_id: i32,
}
