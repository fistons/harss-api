use actix_web::http::HeaderMap;
use actix_web::web::Data;
use actix_web::{dev, web, FromRequest, HttpRequest};
use futures_util::future::{err, ok, Ready};
use hmac::{Hmac, NewMac};
use http_auth_basic::Credentials;
use jwt::SignWithKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::errors::ApiError;
use crate::model::user::User;
use crate::DbPool;
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
struct Claim {
    user: AuthedUser,
    exp: i64,
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
        let (user, password) = match extract_credentials(token) {
            Ok(credentials) => credentials,
            Err(e) => return err(e),
        };
        return match get_and_check_user(&user, &password, pool) {
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

/// # Retrieve a user and check its credentials
fn get_and_check_user(user: &str, password: &str, pool: &Data<DbPool>) -> Result<User, ApiError> {
    let user = crate::services::users::get_user(user, pool)
        .map_err(|_| ApiError::unauthorized("Invalid credentials"))
        .unwrap();

    if !crate::services::users::match_password(&user, &password) {
        return Err(ApiError::unauthorized("Invalid credentials"));
    }

    Ok(user)
}

/// # Return user and password from basic auth value
fn extract_credentials(token: &str) -> Result<(String, String), ApiError> {
    let credentials = Credentials::from_header(token.into()).unwrap();
    Ok((credentials.user_id, credentials.password))
}

/// # Generate a JWT for the given user password
pub fn get_jwt(user: &str, password: &str, pool: &Data<DbPool>) -> Result<String, ApiError> {
    let user = get_and_check_user(user, password, pool)?;

    let authed_user = AuthedUser::from_user(&user);
    let utc: DateTime<Utc> = Utc::now() - Duration::days(1);

    let key: Hmac<Sha256> = Hmac::new_from_slice(b"some-secret").unwrap();

    let claim = Claim {
        user: authed_user,
        exp: utc.timestamp(),
    };

    Ok(claim.sign_with_key(&key)?)
}
