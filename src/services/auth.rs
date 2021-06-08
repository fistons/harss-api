use actix_web::{dev, FromRequest, HttpRequest, web};
use futures_util::future::{err, ok, Ready};
use log::debug;
use serde::Deserialize;

use crate::DbPool;
use crate::errors::ApiError;
use crate::model::user::User;

#[derive(Debug, Deserialize)]
pub struct AuthedUser {
    id: i32,
}

impl AuthedUser {
    pub fn from_user(user: &User) -> Self {
        AuthedUser { id: user.id }
    }
}

impl FromRequest for AuthedUser {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let pool = req.app_data::<web::Data<DbPool>>().unwrap();

        let auth: &str = match req.headers().get("Authorization") {
            None => return err(ApiError::new("No passaran")),
            Some(header) => header.to_str().unwrap() //FIXME: Nope, HeaderValue can be null or all fucked up
        };

        let mut method = auth.split_whitespace();
        if let Some(auth_type) = method.next() {
            let token = match auth_type {
                "Basic" => {
                    debug!("This is a Basic Auth");
                    method.next().unwrap()
                }
                _ => return err(ApiError::new("meh."))
            };

            let creds = String::from_utf8(base64::decode(token).unwrap()).unwrap();


            let creds = creds.split(":").collect::<Vec<&str>>();
            let (user, password) = (creds[0], creds[1]);


            let user = crate::services::users::get_user(user, pool)
                .map_err(|_| ApiError::new("Invalid credentials")).unwrap();

            if !crate::services::users::match_password(&user, password) {
                return err(ApiError::new("Invalid credentials"));
            }

            return ok(AuthedUser::from_user(&user));
        }


        err(ApiError::new("no."))
    }
}