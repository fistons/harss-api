use actix_web::{dev, FromRequest, HttpRequest, web};
use actix_web::web::Data;
use futures_util::future::{err, ok, Ready};
use http_auth_basic::Credentials;
use serde::Deserialize;

use crate::DbPool;
use crate::errors::ApiError;
use crate::model::user::User;

#[derive(Debug, Deserialize)]
pub struct AuthedUser {
    pub id: i32,
    pub login: String,
}

impl AuthedUser {
    pub fn from_user(user: &User) -> Self {
        AuthedUser { id: user.id, login: user.username.clone() }
    }
}

impl FromRequest for AuthedUser {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let pool = req.app_data::<web::Data<DbPool>>().unwrap();

        let token: &str = match req.headers().get("Authorization") {
            None => return err(ApiError::new("No passaran")),
            Some(header) => {
                if let Ok(x) = header.to_str() {
                    x
                } else {
                    return err(ApiError::new("fucked up"));
                }
            }
        };

        return match extract_user_from_basic_auth(token, pool) {
            Ok(u) => ok(AuthedUser::from_user(&u)),
            Err(e) => err(e)
        };
    }
}

fn extract_user_from_basic_auth(token: &str, pool: &Data<DbPool>) -> Result<User, ApiError> {
    let credentials = Credentials::from_header(token.into()).unwrap();
    let user = crate::services::users::get_user(&credentials.user_id, pool)
        .map_err(|_| ApiError::new("Invalid credentials")).unwrap();

    if !crate::services::users::match_password(&user, &credentials.password) {
        return Err(ApiError::new("Invalid credentials"));
    }

    Ok(user)
}