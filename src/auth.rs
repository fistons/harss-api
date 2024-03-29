use std::future::Future;
use std::pin::Pin;

use crate::common::Pool as DbPool;
use actix_web::http::header::HeaderMap;
use actix_web::web::Data;
use actix_web::{dev, FromRequest, HttpRequest};
use anyhow::Context;
use chrono::LocalResult::Single;
use chrono::{DateTime, Duration, TimeZone, Utc};
use deadpool_redis::Pool;
use hmac::{Hmac, Mac};
use http_auth_basic::Credentials;
use jwt::{SignWithKey, VerifyWithKey};
use once_cell::sync::Lazy;
use redis::AsyncCommands;
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::{debug_span, instrument, Instrument};

use crate::common::model::{User, UserRole};
use crate::common::users::*;
use crate::errors::AuthenticationError;

use crate::startup::AppState;

static JWT_KEY: Lazy<Hmac<Sha256>> = Lazy::new(|| {
    Hmac::new_from_slice(
        std::env::var("JWT_SECRET")
            .expect("A JWT_SECRET is mandatory")
            .as_bytes(),
    )
    .unwrap()
});

/// # Represent an authenticated user, from JWT or HTTP Basic Auth
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthenticatedUser {
    pub id: i32,
    pub login: String,
    pub role: UserRole,
}

impl AuthenticatedUser {
    /// # Build an AuthenticatedUser from a SeoORM's model one.
    pub fn from_user(user: &User) -> Self {
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

    #[tracing::instrument(skip_all)]
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

            let app_state = req.app_data::<Data<AppState>>().unwrap();
            check_and_get_authenticated_user(
                &user,
                &password,
                &app_state.db,
                &app_state.redis,
                header_value,
            )
            .await
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
#[instrument(skip(connection, password))]
async fn check_and_get_user(
    connection: &DbPool,
    username: &str,
    password: &Secret<String>,
) -> Result<User, AuthenticationError> {
    let user = match get_user_by_username(connection, username)
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

    if !crate::common::password::verify_password(&user.password, password) {
        return Err(AuthenticationError::Unauthorized(
            "Invalid credentials".into(),
        ));
    }

    Ok(user)
}

/// # Retrieve a user and check its credentials
/// Check in redis if the user has already pass authentication by looking for its header. If not, try to authenticate
/// it by matching password
/// If it's ok, store the header in redis.
/// This is to avoid decoding the password for each request, because it's a costing operation.
#[instrument(skip_all)]
async fn check_and_get_authenticated_user(
    user: &str,
    password: &Secret<String>,
    connection: &DbPool,
    redis: &Pool,
    key: &str,
) -> Result<AuthenticatedUser, AuthenticationError> {
    // Fist, check that the user is not already in the cache
    let mut redis = redis.get().await.context("Couldn't get redis connection")?;
    let value: Option<String> = redis
        .get(format!("user:{}:{}", user, key))
        .instrument(debug_span!("getting_http_token_in_redis"))
        .await
        .context("Could not get value")?;

    if let Some(value) = value {
        // We have something, cool!
        let user: AuthenticatedUser =
            serde_json::from_str(&value).context("Could not deserialize user from redis")?;
        return Ok(user);
    }

    // Nothing? Authenticate dance!
    let user = check_and_get_user(connection, user, password).await?;
    let user = AuthenticatedUser::from_user(&user);

    // Store it in redis
    let serialized_user = serde_json::to_string(&user).context("Could serialize user for redis")?;
    redis
        .set_ex::<_, _, ()>(
            &format!("user:{}:Basic:{}", user.login, key),
            serialized_user,
            60 * 15,
        )
        .instrument(debug_span!("store_http_token_in_redis"))
        .await
        .context("Could not store user in redis")?;

    Ok(user)
}

/// # Return user and password from basic auth value
fn extract_credentials_from_http_basic(
    token: &str,
) -> Result<(String, Secret<String>), AuthenticationError> {
    let credentials = Credentials::from_header(token.into()).unwrap();
    Ok((credentials.user_id, Secret::new(credentials.password)))
}

/// # Generate a JWT for the given user password
#[instrument(skip(connection, password))]
pub async fn get_jwt_from_login_request(
    user: &str,
    password: &Secret<String>,
    connection: &DbPool,
) -> Result<String, AuthenticationError> {
    let user = check_and_get_user(connection, user, password).await?;

    get_jwt(&user).await
}

/// # Generate a JWT for the given user
pub async fn get_jwt(user: &User) -> Result<String, AuthenticationError> {
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

#[instrument(skip_all)]
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
