use std::env;

use crate::common::password::verify_password;
use crate::common::DbError::RowNotFound;
use actix_web::{get, patch, post, web, HttpResponse};
use secrecy::ExposeSecret;
use serde_json::json;

use crate::common::model::UserRole;
use crate::common::users::{self, get_user_by_id};

use crate::auth::AuthenticatedUser;
use crate::errors::AuthenticationError;
use crate::model::{
    NewUserRequest, PageParameters, ResetPasswordRequest, UpdateOtherPasswordRequest,
    UpdatePasswordRequest,
};
use crate::routes::errors::ApiError;
use crate::startup::AppState;

#[post("/users")]
#[tracing::instrument(skip(app_state))]
async fn new_user(
    new_user: web::Json<NewUserRequest>,
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

        let user = users::create_user(
            connection,
            &data.username,
            &data.password,
            data.email,
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
        Ok(HttpResponse::Ok()
            .json(users::list_users(connection, page.get_page(), page.get_size()).await?))
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

    if let Ok(Some(user)) = get_user_by_id(connection, user.id).await {
        if !verify_password(&user.password, &request.current_password) {
            return Err(ApiError::PasswordMismatch);
        }
    } else {
        return Err(ApiError::NotFound("User".to_owned(), user.id));
    }

    if let Err(e) = users::update_user_password(connection, user.id, &request.new_password).await {
        return Err(ApiError::DatabaseError(e));
    }
    //TODO: Invalid token?
    Ok(HttpResponse::NoContent().finish())
}

#[post("/user/reset-password")]
#[tracing::instrument(skip(app_state), level = "debug")]
async fn reset_password(
    app_state: web::Data<AppState>,
    request: web::Json<ResetPasswordRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;

    let _ = users::reset_password(connection, redis, &request.email).await;

    Ok(HttpResponse::Ok().finish())
}

#[patch("/user/{user_id}/update-password")]
#[tracing::instrument(skip(app_state), level = "debug")]
async fn update_other_password(
    app_state: web::Data<AppState>,
    user_id: web::Path<i32>,
    request: web::Json<UpdateOtherPasswordRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let user_id = user_id.into_inner();

    if !user.is_admin() {
        return Err(ApiError::AuthenticationError(
            AuthenticationError::Forbidden("You need to be an administrator".to_owned()),
        ));
    }

    if request.new_password.expose_secret() != request.confirm_password.expose_secret() {
        return Err(ApiError::PasswordMismatch);
    }

    if let Err(e) = users::update_user_password(connection, user_id, &request.new_password).await {
        return match e {
            RowNotFound => Err(ApiError::NotFound(String::from("user"), user_id)),
            _ => return Err(ApiError::DatabaseError(e)),
        };
    }
    //TODO: We should probably invalid the current refresh token in redis
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(new_user)
        .service(list_users)
        .service(update_password)
        .service(update_other_password)
        .service(reset_password);
}
