use actix_web::web;

pub mod auth;
pub mod channels;
pub mod items;
pub mod users;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure)
        .configure(channels::configure)
        .configure(items::configure)
        .configure(users::configure);
}
