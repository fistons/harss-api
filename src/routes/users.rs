use std::env;

use crate::common::password::verify_password;
use crate::common::DbError::RowNotFound;
use actix_web::{delete, get, patch, post, web, HttpResponse};
use secrecy::{ExposeSecret, Secret};
use serde_json::json;
use tracing::debug;

use crate::common::model::UserRole;
use crate::common::users::{self, get_user_by_id};

use crate::auth::AuthenticatedUser;
use crate::errors::AuthenticationError;
use crate::model::{
    NewUserRequest, PageParameters, ResetPasswordRequest, ResetPasswordTokenRequest,
    UpdateOtherPasswordRequest, UpdatePasswordRequest, UpdateUserRequest,
};
use crate::routes::errors::ApiError;
use crate::startup::AppState;

#[post("/users")]
async fn new_user(
    request: web::Json<NewUserRequest>,
    app_state: web::Data<AppState>,
    user: Option<AuthenticatedUser>,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;

    let admin = user.map(|x| x.is_admin()).unwrap_or(false);
    let allow_account_creation = env::var("RSS_AGGREGATOR_ALLOW_ACCOUNT_CREATION")
        .map(|x| x.parse().unwrap_or_default())
        .unwrap_or_default();

    if allow_account_creation || admin {
        debug!("Recording new user {:?}", request);

        if request.role == UserRole::Admin && !admin {
            debug!("Tried to create a new admin with a non admin user");
            return Ok(HttpResponse::Unauthorized().finish());
        }

        if request.password.expose_secret() != request.confirm_password.expose_secret() {
            return Err(ApiError::PasswordMismatch);
        }

        let user = users::create_user(
            redis,
            connection,
            &request.username,
            &request.password,
            &request.email,
            &request.role,
        )
        .await?;

        Ok(HttpResponse::Created().json(json!({"id": user.id})))
    } else {
        debug!("User creation attempt while it's disabled or creator is not admin");
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[get("/users")]
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
async fn update_password(
    app_state: web::Data<AppState>,
    request: web::Json<UpdatePasswordRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;

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

    users::delete_user_redis_keys(redis, user.id).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/user/reset-password-request")]
async fn reset_password_token(
    app_state: web::Data<AppState>,
    request: web::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;

    users::reset_password_request(connection, redis, &request.email).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[post("/user/reset-password")]
async fn reset_password(
    app_state: web::Data<AppState>,
    request: web::Json<ResetPasswordTokenRequest>,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;

    if request.new_password.expose_secret() != request.confirm_password.expose_secret() {
        return Err(ApiError::PasswordMismatch);
    }

    users::reset_password(
        connection,
        redis,
        &request.token,
        &request.new_password,
        &request.username,
    )
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[patch("/user/{user_id}/update-password")]
async fn update_other_password(
    app_state: web::Data<AppState>,
    user_id: web::Path<i32>,
    request: web::Json<UpdateOtherPasswordRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;
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

    users::delete_user_redis_keys(redis, user_id).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[patch("/user")]
async fn update_user(
    app_state: web::Data<AppState>,
    request: web::Json<UpdateUserRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;

    users::update_user(connection, redis, user.id, &request.email).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[get("/user/confirm-email/{token}")]
async fn confirm_email(
    app_state: web::Data<AppState>,
    token: web::Path<Secret<String>>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let connection = &app_state.db;
    let redis = &app_state.redis;

    users::confirm_email(connection, redis, user.id, &token).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[delete("/user/{user_id}")]
async fn delete_user(
    app_state: web::Data<AppState>,
    user_id: web::Path<i32>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let db = &app_state.db;
    let redis = &app_state.redis;

    if !user.is_admin() && user.id != *user_id {
        return Err(ApiError::NotFound("User not found".to_owned(), *user_id));
    }

    users::delete_user(db, redis, *user_id).await?;

    Ok(HttpResponse::NoContent().json(json!({"you-will-be":"missed"})))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(new_user)
        .service(list_users)
        .service(update_password)
        .service(update_other_password)
        .service(reset_password_token)
        .service(update_user)
        .service(confirm_email)
        .service(delete_user)
        .service(reset_password);
}
