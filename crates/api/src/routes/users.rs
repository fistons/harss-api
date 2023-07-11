use std::env;

use actix_web::{get, patch, post, web, HttpResponse};
use secrecy::ExposeSecret;
use serde_json::json;

use rss_common::model::{
    HttpNewUser, PageParameters, PagedResult, UpdateOtherPasswordRequest, UpdatePasswordRequest,
};
use rss_common::services::users::UserService;
use rss_common::services::AuthenticationError;
use rss_common::UserRole;

use crate::auth::AuthenticatedUser;
use crate::routes::ApiError;
use crate::startup::AppState;

#[post("/users")]
#[tracing::instrument(skip(app_state))]
async fn new_user(
    new_user: web::Json<HttpNewUser>,
    app_state: web::Data<AppState>,
    user: Option<AuthenticatedUser>,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    let admin = user.map(|x| x.is_admin()).unwrap_or(false);
    let allow_account_creation = env::var("RSS_AGGREGATOR_ALLOW_ACCOUNT_CREATION")
        .map(|x| x.parse().unwrap_or_default())
        .unwrap_or_default();

    if allow_account_creation || admin {
        tracing::debug!("Recording new user {:?}", new_user);
        let data = new_user.into_inner();

        if data.role == UserRole::Admin && !admin {
            tracing::debug!("Tried to create a new admin with a non admin user");
            return Ok(HttpResponse::Unauthorized().finish());
        }

        let user = UserService::create_user(
            connection,
            &data.username,
            data.password.expose_secret(),
            data.role,
        )
        .await?;

        Ok(HttpResponse::Created().json(json!({"id": user.id})))
    } else {
        tracing::debug!("User creation attempt while it's disabled or creator is not admin");
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[get("/users")]
#[tracing::instrument(skip(app_state))]
async fn list_users(
    app_state: web::Data<AppState>,
    page: web::Query<PageParameters>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    if user.is_admin() {
        let users_page =
            UserService::list_users(connection, page.get_page(), page.get_size()).await?;
        let users = PagedResult {
            content: users_page.content,
            page: users_page.page,
            page_size: users_page.page_size,
            total_pages: users_page.total_pages,
            elements_number: users_page.elements_number,
            total_items: users_page.total_items,
        };

        Ok(HttpResponse::Ok().json(users))
    } else {
        Err(ApiError::AuthenticationError(
            AuthenticationError::Forbidden("no".into()),
        ))
    }
}

#[patch("/user/update-password")]
#[tracing::instrument(skip(app_state), level = "debug")]
async fn update_password(
    app_state: web::Data<AppState>,
    request: web::Json<UpdatePasswordRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    if request.new_password.expose_secret() != request.confirm_password.expose_secret() {
        return Err(ApiError::PasswordMismatch);
    }

    if let Err(e) = UserService::update_password(
        connection,
        user.id,
        &request.current_password,
        &request.new_password,
    )
    .await
    {
        return Err(ApiError::ServiceError(e));
    }
    //TODO: Invalid token?
    Ok(HttpResponse::NoContent().finish())
}

#[patch("/user/{user_id}/update-password")]
#[tracing::instrument(skip(app_state), level = "debug")]
//FIXME Something fishy is in here
async fn update_other_password(
    app_state: web::Data<AppState>,
    user_id: web::Path<i32>,
    request: web::Json<UpdateOtherPasswordRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;

    if !user.is_admin() {
        return Err(ApiError::AuthenticationError(
            AuthenticationError::Forbidden("no".into()),
        ));
    }

    if request.new_password.expose_secret() != request.confirm_password.expose_secret() {
        return Err(ApiError::PasswordMismatch);
    }

    if let Err(e) =
        UserService::update_other_user_password(connection, user.id, &request.new_password).await
    {
        return Err(ApiError::ServiceError(e));
    }
    //TODO: Invalid token?
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(new_user)
        .service(list_users)
        .service(update_password);
}
