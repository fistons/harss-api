use std::env;

use actix_web::{get, patch, post, web, HttpResponse};
use secrecy::ExposeSecret;
use serde_json::json;

use entity::sea_orm_active_enums::UserRole;

use crate::model::{HttpNewUser, PageParameters, PagedResult, UpdatePasswordRequest};
use crate::routes::ApiError;
use crate::services::auth::AuthenticatedUser;
use crate::services::AuthenticationError;
use crate::startup::ApplicationServices;

#[post("/users")]
#[tracing::instrument(skip(services))]
async fn new_user(
    new_user: web::Json<HttpNewUser>,
    services: web::Data<ApplicationServices>,
    user: Option<AuthenticatedUser>,
) -> Result<HttpResponse, ApiError> {
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

        let user = services
            .user_service
            .create_user(&data.username, data.password.expose_secret(), data.role)
            .await?;

        Ok(HttpResponse::Created().json(json!({"id": user.id})))
    } else {
        tracing::debug!("User creation attempt while it's disabled or creator is not admin");
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[get("/users")]
#[tracing::instrument(skip(services))]
async fn list_users(
    services: web::Data<ApplicationServices>,
    page: web::Query<PageParameters>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    if user.is_admin() {
        let users_page = services
            .user_service
            .list_users(page.get_page(), page.get_size())
            .await?;
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
#[tracing::instrument(skip(services), level = "debug")]
async fn update_password(
    services: web::Data<ApplicationServices>,
    request: web::Json<UpdatePasswordRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    if let Err(e) = services
        .user_service
        .update_password(
            user.id,
            &request.current_password,
            &request.new_password,
            &request.confirm_password,
        )
        .await
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
