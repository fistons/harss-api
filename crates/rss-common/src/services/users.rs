use sea_orm::DbErr;
use sea_orm::{entity::*, query::*};
use secrecy::{ExposeSecret, Secret};

use entity::sea_orm_active_enums::UserRole;
use entity::users;
use entity::users::Entity as User;

use crate::model::{HttpUser, PagedResult};
use crate::services::password::{encode_password, match_password};
use crate::services::ServiceError;
use crate::services::ServiceError::NonMatchingPassword;

pub struct UserService;

impl UserService {
    #[tracing::instrument(skip(db))]
    pub async fn get_user<C>(db: &C, wanted_username: &str) -> Result<Option<users::Model>, DbErr>
    where
        C: ConnectionTrait,
    {
        User::find()
            .filter(users::Column::Username.eq(wanted_username))
            .one(db)
            .await
    }

    #[tracing::instrument(skip(db), level = "debug")]
    pub async fn get_user_by_id<C>(db: &C, id: i32) -> Result<Option<users::Model>, DbErr>
    where
        C: ConnectionTrait,
    {
        User::find().filter(users::Column::Id.eq(id)).one(db).await
    }

    #[tracing::instrument(skip(db))]
    pub async fn list_users<C>(
        db: &C,
        page: u64,
        page_size: u64,
    ) -> Result<PagedResult<HttpUser>, DbErr>
    where
        C: ConnectionTrait,
    {
        let users_pagination = User::find()
            .into_model::<HttpUser>()
            .paginate(db, page_size);

        let total_pages = users_pagination.num_pages().await?;
        let total_items = users_pagination.num_items().await?;
        let content = users_pagination.fetch_page(page - 1).await?;
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

    #[tracing::instrument(skip(db))]
    pub async fn create_user<C>(
        db: &C,
        login: &str,
        pwd: &str,
        user_role: UserRole,
    ) -> Result<users::Model, DbErr>
    where
        C: ConnectionTrait,
    {
        let new_user = users::ActiveModel {
            id: NotSet,
            username: Set(String::from(login)),
            password: Set(encode_password(pwd)),
            role: Set(user_role),
        };

        new_user.insert(db).await
    }

    //TODO: improve errors
    #[tracing::instrument(skip(db))]
    pub async fn update_password<C>(
        db: &C,
        user_id: i32,
        current_password: &Secret<String>,
        new_password: &Secret<String>,
    ) -> Result<(), ServiceError>
    where
        C: ConnectionTrait,
    {
        let user = UserService::get_user_by_id(db, user_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("User not found".to_owned()))?;

        if !match_password(&user, current_password.expose_secret()) {
            return Err(NonMatchingPassword);
        }

        let mut user: users::ActiveModel = user.into();
        user.password = Set(encode_password(new_password.expose_secret()));
        user.update(db).await?;

        Ok(())
    }

    //TODO: improve errors
    #[tracing::instrument(skip(db))]
    pub async fn update_other_user_password<C>(
        db: &C,
        user_id: i32,
        new_password: &Secret<String>,
    ) -> Result<(), ServiceError>
    where
        C: ConnectionTrait,
    {
        let user = UserService::get_user_by_id(db, user_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("User not found".to_owned()))?;

        let mut user: users::ActiveModel = user.into();
        user.password = Set(encode_password(new_password.expose_secret()));
        user.update(db).await?;

        Ok(())
    }
}
