use actix_web::http::HeaderMap;
use actix_web::web::Data;
use actix_web::{dev, web, FromRequest, HttpRequest};
use futures_util::future::{err, ok, Ready};
use http_auth_basic::Credentials;
use serde::Deserialize;

use crate::errors::ApiError;
use crate::model::user::User;
use crate::DbPool;

#[derive(Debug, Deserialize)]
pub struct AuthedUser {
    pub id: i32,
    pub login: String,
}

impl AuthedUser {
    pub fn from_user(user: &User) -> Self {
        AuthedUser {
            id: user.id,
            login: user.username.clone(),
        }
    }
}

impl FromRequest for AuthedUser {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let pool = req.app_data::<web::Data<DbPool>>().unwrap();

        let token = match extract_value_authentication_header(req.headers()) {
            Ok(header) => header,
            Err(e) => return err(e),
        };

        return match extract_user_from_basic_auth(token, pool) {
            Ok(u) => ok(AuthedUser::from_user(&u)),
            Err(e) => err(e),
        };
    }
}

/// # Extract the authentication string form the Header
fn extract_value_authentication_header(headers: &HeaderMap) -> Result<&str, ApiError> {
    let token: &str = match headers.get("Authorization") {
        None => return Err(ApiError::default("No passaran")),
        Some(header) => header.to_str().map_err(|_| ApiError::default("meh"))?,
    };

    Ok(token)
}

/// # Verify user credentials
fn extract_user_from_basic_auth(token: &str, pool: &Data<DbPool>) -> Result<User, ApiError> {
    let credentials = Credentials::from_header(token.into()).unwrap();
    let user = crate::services::users::get_user(&credentials.user_id, pool)
        .map_err(|_| ApiError::unauthorized("Invalid credentials"))
        .unwrap();

    if !crate::services::users::match_password(&user, &credentials.password) {
        return Err(ApiError::unauthorized("Invalid credentials"));
    }

    Ok(user)
}
