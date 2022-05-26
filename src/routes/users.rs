use actix_web::{get, post, web, HttpResponse};
use serde_json::json;

use entity::sea_orm_active_enums::UserRole;

use crate::errors::ApiError;
use crate::model::configuration::ApplicationConfiguration;
use crate::model::{HttpNewUser, PageParameters, PagedResult};
use crate::services::auth::AuthenticatedUser;
use crate::services::users::UserService;

#[post("/users")]
async fn new_user(
    new_user: web::Json<HttpNewUser>,
    user_service: web::Data<UserService>,
    user: Option<AuthenticatedUser>,
    configuration: web::Data<ApplicationConfiguration>,
) -> Result<HttpResponse, ApiError> {
    let admin = user.map(|x| x.is_admin()).unwrap_or(false);
    if configuration.allow_account_creation.unwrap_or(false) || admin {
        log::debug!("Recording new user {:?}", new_user);
        let data = new_user.into_inner();

        if data.role == UserRole::Admin && admin {
            log::debug!("Tried to create a new admin with a non admin user");
            return Ok(HttpResponse::Unauthorized().finish());
        }

        let user = user_service
            .create_user(&data.username, &data.password, data.role)
            .await?;

        Ok(HttpResponse::Created().json(json!({"id": user.id})))
    } else {
        log::debug!("User creation attempt while it's disabled or creator is not admin");
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[get("/users")]
async fn list_users(
    user_service: web::Data<UserService>,
    page: web::Query<PageParameters>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    if user.is_admin() {
        log::debug!("Get all users");

        let users_page = user_service
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
        Err(ApiError::unauthorized("You are not allowed to do that"))
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(new_user).service(list_users);
}
