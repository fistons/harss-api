use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use sea_orm::{entity::*, query::*};
use sea_orm::{DatabaseConnection, DbErr};
use secrecy::{ExposeSecret, Secret};

use entity::sea_orm_active_enums::UserRole;
use entity::users;
use entity::users::Entity as User;

use crate::model::{HttpUser, PagedResult};
use crate::services::ServiceError;
use crate::services::ServiceError::NonMatchingPassword;

#[derive(Clone)]
pub struct UserService {
    db: DatabaseConnection,
}

impl UserService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_user(&self, wanted_username: &str) -> Result<Option<users::Model>, DbErr> {
        User::find()
            .filter(users::Column::Username.eq(wanted_username))
            .one(&self.db)
            .await
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_user_by_id(&self, id: i32) -> Result<Option<users::Model>, DbErr> {
        User::find()
            .filter(users::Column::Id.eq(id))
            .one(&self.db)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_users(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PagedResult<HttpUser>, DbErr> {
        let user_paginator = User::find()
            .into_model::<HttpUser>()
            .paginate(&self.db, page_size);

        let total_pages = user_paginator.num_pages().await?;
        let total_items = user_paginator.num_items().await?;
        let content = user_paginator.fetch_page(page - 1).await?;
        let elements_number = content.len();

        Ok(PagedResult {
            content,
            page,
            page_size,
            total_pages,
            elements_number,
            total_items,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_user(
        &self,
        login: &str,
        pwd: &str,
        user_role: UserRole,
    ) -> Result<users::Model, DbErr> {
        let new_user = users::ActiveModel {
            id: NotSet,
            username: Set(String::from(login)),
            password: Set(encode_password(pwd)),
            role: Set(user_role),
        };

        new_user.insert(&self.db).await
    }

    //TODO: improve errors
    #[tracing::instrument(skip(self))]
    pub async fn update_password(
        &self,
        user_id: i32,
        current_password: &Secret<String>,
        new_password: &Secret<String>,
    ) -> Result<(), ServiceError> {
        let user = self
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("User not found".to_owned()))?;

        if !match_password(&user, current_password.expose_secret()) {
            return Err(NonMatchingPassword);
        }

        let mut user: users::ActiveModel = user.into();
        user.password = Set(encode_password(new_password.expose_secret()));
        user.update(&self.db).await?;

        Ok(())
    }

    //TODO: improve errors
    #[tracing::instrument(skip(self))]
    pub async fn update_other_user_password(
        &self,
        user_id: i32,
        new_password: &Secret<String>,
    ) -> Result<(), ServiceError> {
        let user = self
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("User not found".to_owned()))?;

        let mut user: users::ActiveModel = user.into();
        user.password = Set(encode_password(new_password.expose_secret()));
        user.update(&self.db).await?;

        Ok(())
    }
}

#[tracing::instrument(skip(pwd))]
fn encode_password(pwd: &str) -> String {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon2
        .hash_password(pwd.as_bytes(), &salt)
        .unwrap()
        .to_string();

    password_hash
}

#[tracing::instrument(skip_all)]
pub fn match_password(user: &users::Model, candidate: &str) -> bool {
    let parsed_hash = PasswordHash::new(&user.password).unwrap();
    Argon2::default()
        .verify_password(candidate.as_bytes(), &parsed_hash)
        .is_ok()
}
