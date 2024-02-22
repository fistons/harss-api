use crate::common::model::{PagedResult, User, UserRole};
use crate::common::password::encode_password;
use secrecy::Secret;
use sqlx::PgPool;
use sqlx::Result;
use tracing::{info, instrument};

pub struct UserDao {
    db: PgPool,
}

impl UserDao {
    pub fn new(db: PgPool) -> Self {
        UserDao { db }
    }

    #[instrument(skip(self))]
    pub async fn get_user_by_username(&self, wanted_username: &str) -> Result<Option<User>> {
        sqlx::query_as!(
        User,
        r#"
            SELECT id, username, password, role as "role: UserRole", email_verified FROM users WHERE username = $1
        "#,
        wanted_username
        )
        .fetch_optional(&self.db)
        .await
    }

    /// Return the user matching the id
    #[instrument(skip(self))]
    pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>> {
        sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole", email_verified FROM users WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&self.db)
    .await
    }

    /// Return the user matching the id
    #[instrument(skip(self))]
    pub async fn get_user_by_hashed_email(&self, hashed_email: &str) -> Result<Option<User>> {
        sqlx::query_as!(
        User,
        r#"
        SELECT id, username, password, role as "role: UserRole", email_verified FROM users WHERE email = $1 AND email_verified = true
        "#,
        hashed_email
    )
    .fetch_optional(&self.db)
    .await
    }

    /// List all the users
    #[instrument(skip(self))]
    pub async fn list_users(&self, page_number: u64, page_size: u64) -> Result<PagedResult<User>> {
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
        .fetch_all(&self.db)
        .await?;

        let total_items = sqlx::query_scalar!(
            r#"
        SELECT COUNT(*) FROM users
        "#,
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

    /// Create a new user
    #[instrument(skip(self, password))]
    pub async fn create_user(
        &self,
        login: &str,
        password: &Secret<String>,
        hashed_email: &Option<String>,
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
        hashed_email.to_owned(),
        user_role as &UserRole
    )
    .fetch_one(&self.db)
    .await?; //TODO Make a beautifull error on unique constraint violation

        Ok(user)
    }

    /// Update a user's password
    #[instrument(skip(self, new_password))]
    pub async fn update_user_password(
        &self,
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
        .execute(&self.db)
        .await?;

        //TODO: Must return a dedicated error
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    #[instrument(skip(self, hashed_email))]
    pub async fn update_user(
        &self,
        user_id: i32,
        hashed_email: &Option<String>,
    ) -> anyhow::Result<()> {
        let user = self
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        if let Some(email) = hashed_email {
            sqlx::query!(
                r#"UPDATE users SET email = $1, email_verified = false WHERE id = $2"#,
                email,
                user.id
            )
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn delete_user(&self, user_id: i32) -> anyhow::Result<()> {
        let result = sqlx::query!(r#"DELETE FROM users WHERE id = $1"#, user_id)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound)?;
        }

        info!("Deleted user {}", user_id);
        Ok(())
    }
}
