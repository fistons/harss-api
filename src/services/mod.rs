use std::sync::Arc;

use actix_web::{Error, web};
use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::basic::BasicAuth;
use log::debug;

use crate::{DbPool, services};
use crate::errors::ApiError;

pub mod channels;
pub mod items;
pub mod users;

pub fn refresh(pool: &Arc<DbPool>) -> Result<(), diesel::result::Error> {
    let channels = crate::services::channels::select_all(pool)?;

    for channel in channels.iter() {
        refresh_chan(pool, channel.id)?;
    }
    Ok(())
}

pub fn refresh_chan(pool: &Arc<DbPool>, channel_id: i32) -> Result<(), diesel::result::Error> {
    let channel = crate::services::channels::select_by_id(channel_id, pool)?;
    debug!("Fetching {}", &channel.name);

    let content = reqwest::blocking::get(&channel.url)
        .unwrap()
        .bytes()
        .unwrap();
    let rss_channel = rss::Channel::read_from(&content[..]).unwrap();
    for item in rss_channel.items.into_iter() {
        let i = crate::model::item::NewItem::from_rss_item(item, channel.id);
        services::items::insert(i, pool).unwrap();
    }

    Ok(())
}

pub async fn validator(req: ServiceRequest, creds: BasicAuth) -> Result<ServiceRequest, actix_web::Error> {
    let pool = req.app_data::<web::Data<DbPool>>().unwrap();

    let user = creds.user_id();
    let password = match creds.password() {
        Some(pass) => pass,
        _ => return Err(Error::from(ApiError::new(String::from("Password is mandatory"))))
    };

    let user = crate::services::users::get_user(user, pool)
        .map_err(|_| ApiError::new(String::from("Invalid credentials")))?;

    if !crate::services::users::match_password(&user, password) {
        return Err(Error::from(ApiError::new(String::from("Invalid credentials"))));
    }

    Ok(req)
}
