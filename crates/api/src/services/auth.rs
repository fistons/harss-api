use std::future::Future;
use std::pin::Pin;

use actix_web::http::header::HeaderMap;
use actix_web::web::Data;
use actix_web::{dev, FromRequest, HttpRequest};
use anyhow::Context;
use chrono::LocalResult::Single;
use chrono::{DateTime, Duration, TimeZone, Utc};
use hmac::{Hmac, Mac};
use http_auth_basic::Credentials;
use jwt::{SignWithKey, VerifyWithKey};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use entity::sea_orm_active_enums::UserRole;
use entity::users;

use crate::services::users::UserService;
use crate::services::AuthenticationError;
use crate::startup::ApplicationServices;

lazy_static! {
    /// Generate the JWT key at runtime. If the JWT_SECRET environment variable is not set,
    /// fail miserably
    static ref JWT_KEY: Hmac<Sha256> =  Hmac::new_from_slice(std::env::var("JWT_SECRET")
        .expect("A JWT_SECRET is mandatory")
        .as_bytes()).unwrap();
}

/// # Represent an authenticated user, from JWT or HTTP Basic Auth
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthenticatedUser {
    pub id: i32,
    pub login: String,
    pub role: UserRole,
}

impl AuthenticatedUser {
    /// # Build an AuthenticatedUser from a SeoORM's model one.
    pub fn from_user(user: &users::Model) -> Self {
        AuthenticatedUser {
            id: user.id,
            login: user.username.clone(),
            role: user.role.clone(),
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }
}

#[derive(Debug, Deserialize, Serialize)]
/// # JWT claims
struct Claims {
    user: AuthenticatedUser,
    exp: i64,
}

impl FromRequest for AuthenticatedUser {
    type Error = AuthenticationError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    #[tracing::instrument(skip_all, level = "trace")]
    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move { extract_authenticated_user(&req).await })
    }
}

/// # Extract the authenticated user from the request
async fn extract_authenticated_user(
    req: &HttpRequest,
) -> Result<AuthenticatedUser, AuthenticationError> {
    let req = req.clone();
    let header_value = match extract_value_authentication_header(req.headers()) {
        Ok(header) => header,
        Err(e) => return Err(e),
    };

    let mut split_header = header_value.split_whitespace();
    let scheme = split_header.next().unwrap();
    let value = if let Some(token) = split_header.next() {
        token
    } else {
        return Err(AuthenticationError::Unauthorized(
            "Invalid Authorization header value".into(),
        ));
    };

    let req = req.clone();

    return match (scheme, value) {
        (bearer, token) if bearer.to_ascii_lowercase() == "bearer" => verify_jwt(token).await,
        (basic, _) if basic.to_ascii_lowercase() == "basic" => {
            let (user, password) = match extract_credentials_from_http_basic(header_value) {
                Ok(credentials) => credentials,
                Err(e) => return Err(e),
            };

            let services = req.app_data::<Data<ApplicationServices>>().unwrap();
            check_and_get_authed_user(&user, &password, &services.user_service).await
        }

        (_error, _) => Err(AuthenticationError::UnknownAuthScheme),
    };
}

/// # Extract the authentication string form the Header
fn extract_value_authentication_header(headers: &HeaderMap) -> Result<&str, AuthenticationError> {
    let token: &str = match headers.get("Authorization") {
        None => {
            return Err(AuthenticationError::Unauthorized(
                "Missing Authorization header value".into(),
            ))
        }
        Some(header) => header.to_str().map_err(|x| {
            AuthenticationError::Unauthorized(format!("Invalid Authentication header value: {}", x))
        })?,
    };

    Ok(token)
}

/// # Retrieve a user and check its credentials
async fn check_and_get_user(
    user: &str,
    password: &str,
    user_service: &UserService,
) -> Result<users::Model, AuthenticationError> {
    let user = match user_service
        .get_user(user)
        .await
        .context("Database error")?
    {
        None => {
            return Err(AuthenticationError::Unauthorized(
                "Invalid credentials".into(),
            ))
        }
        Some(u) => u,
    };

    if !crate::services::users::match_password(&user, password) {
        return Err(AuthenticationError::Unauthorized(
            "Invalid credentials".into(),
        ));
    }

    Ok(user)
}

/// # Retrieve a user and check its credentials
async fn check_and_get_authed_user(
    user: &str,
    password: &str,
    user_service: &UserService,
) -> Result<AuthenticatedUser, AuthenticationError> {
    let user = check_and_get_user(user, password, user_service).await?;
    Ok(AuthenticatedUser::from_user(&user))
}

/// # Return user and password from basic auth value
fn extract_credentials_from_http_basic(
    token: &str,
) -> Result<(String, String), AuthenticationError> {
    let credentials = Credentials::from_header(token.into()).unwrap();
    Ok((credentials.user_id, credentials.password))
}

/// # Generate a JWT for the given user password
pub async fn get_jwt_from_login_request(
    user: &str,
    password: &str,
    user_service: &UserService,
) -> Result<String, AuthenticationError> {
    let user = check_and_get_user(user, password, user_service).await?;

    get_jwt(&user).await
}

/// # Generate a JWT for the given user
pub async fn get_jwt(user: &users::Model) -> Result<String, AuthenticationError> {
    let utc: DateTime<Utc> = Utc::now() + Duration::minutes(15);
    let authenticated_user = AuthenticatedUser::from_user(user);

    let claim = Claims {
        user: authenticated_user,
        exp: utc.timestamp(),
    };

    Ok(claim.sign_with_key(&(*JWT_KEY))?)
}

pub fn extract_login_from_refresh_token(token: &str) -> &str {
    token.split('.').collect::<Vec<&str>>()[1]
}

async fn verify_jwt(token: &str) -> Result<AuthenticatedUser, AuthenticationError> {
    let claims: Claims = token.verify_with_key(&(*JWT_KEY))?;

    let date = if let Single(t) = Utc.timestamp_opt(claims.exp, 0) {
        t
    } else {
        return Err(AuthenticationError::ExpiredToken);
    };

    if date.lt(&Utc::now()) {
        return Err(AuthenticationError::ExpiredToken);
    }
    Ok(claims.user)
}
