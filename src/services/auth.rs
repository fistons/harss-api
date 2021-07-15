use actix_web::http::HeaderMap;
use actix_web::web::Data;
use actix_web::{dev, web, FromRequest, HttpRequest};
use chrono::{DateTime, Duration, TimeZone, Utc};
use futures_util::future::{err, ok, Ready};
use hmac::{Hmac, NewMac};
use http_auth_basic::Credentials;
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::errors::ApiError;
use crate::model::user::{User, UserRole};
use crate::services::users::UserService;

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthedUser {
    pub id: i32,
    pub login: String,
    pub role: UserRole
}

impl AuthedUser {
    pub fn from_user(user: &User) -> Self {
        AuthedUser {
            id: user.id,
            login: user.username.clone(),
            role: user.role.clone()
        }
    }
    
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    user: AuthedUser,
    exp: i64,
}

impl FromRequest for AuthedUser {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let user_service = req.app_data::<web::Data<UserService>>().unwrap();

        let header_value = match extract_value_authentication_header(req.headers()) {
            Ok(header) => header,
            Err(e) => return err(e),
        };

        let mut split_header = header_value.split_whitespace();
        let scheme = split_header.next().unwrap();
        let value = if let Some(token) = split_header.next() {
            token
        } else {
            return err(ApiError::unauthorized("Invalid Authorization header value"));
        };
        return match (scheme, value) {
            (bearer, token) if bearer.to_ascii_lowercase() == "bearer" => match verify_jwt(token) {
                Ok(user) => ok(user),
                Err(e) => err(e),
            },
            (basic, _) if basic.to_ascii_lowercase() == "basic" => {
                let (user, password) = match extract_credentials_from_http_basic(header_value) {
                    Ok(credentials) => credentials,
                    Err(e) => return err(e),
                };
                match get_and_check_user(&user, &password, &user_service) {
                    Ok(u) => ok(AuthedUser::from_user(&u)),
                    Err(e) => err(e),
                }
            }
            (error, _) => err(ApiError::unauthorized(format!(
                "Unknown Authorization scheme: {}",
                error
            ))),
        };
    }
}

/// # Extract the authentication string form the Header
fn extract_value_authentication_header(headers: &HeaderMap) -> Result<&str, ApiError> {
    let token: &str = match headers.get("Authorization") {
        None => return Err(ApiError::unauthorized("Missing Authorization header value")),
        Some(header) => header.to_str().map_err(|x| {
            ApiError::unauthorized(format!("Invalid Authentication header value: {}", x))
        })?,
    };

    Ok(token)
}

/// # Retrieve a user and check its credentials
fn get_and_check_user(
    user: &str,
    password: &str,
    user_service: &UserService,
) -> Result<User, ApiError> {
    let user = user_service
        .get_user(user)
        .map_err(|_| ApiError::unauthorized("Invalid credentials"))
        .unwrap();

    if !crate::services::users::match_password(&user, &password) {
        return Err(ApiError::unauthorized("Invalid credentials"));
    }

    Ok(user)
}

/// # Return user and password from basic auth value
fn extract_credentials_from_http_basic(token: &str) -> Result<(String, String), ApiError> {
    let credentials = Credentials::from_header(token.into()).unwrap();
    Ok((credentials.user_id, credentials.password))
}

/// # Generate a JWT for the given user password
pub fn get_jwt_from_login_request(
    user: &str,
    password: &str,
    user_service: Data<UserService>,
) -> Result<String, ApiError> {
    let user = get_and_check_user(user, password, &user_service)?;

    get_jwt(&user)
}

/// # Generate a JWT for the given user
pub fn get_jwt(user: &User) -> Result<String, ApiError> {
    let utc: DateTime<Utc> = Utc::now() + Duration::minutes(15);
    let key: Hmac<Sha256> = Hmac::new_from_slice(get_jwt_secret().as_bytes()).unwrap();

    let authed_user = AuthedUser::from_user(user);

    let claim = Claims {
        user: authed_user,
        exp: utc.timestamp(),
    };

    Ok(claim.sign_with_key(&key)?)
}

pub fn extract_login_from_refresh_token(token: &str) -> &str {
    token.split('.').collect::<Vec<&str>>()[1]
}

fn verify_jwt(token: &str) -> Result<AuthedUser, ApiError> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(get_jwt_secret().as_bytes()).unwrap();
    let claims: Claims = token.verify_with_key(&key)?;

    let date = Utc.timestamp(claims.exp, 0);
    if date.lt(&Utc::now()) {
        return Err(ApiError::unauthorized("Token is expired, go home."));
    }
    Ok(claims.user)
}

fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| String::from("aecda4f3-08a2-43e4-8b42-575455ade8b0"))
}
