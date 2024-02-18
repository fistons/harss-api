use std::net::TcpListener;

use actix_governor::Governor;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use deadpool_redis::Pool;

use crate::common::Pool as DbPool;

use crate::rate_limiting::build_rate_limiting_conf;
use crate::routes;
use crate::services::channels::ChannelService;
use crate::services::users::UserService;

pub struct AppState {
    pub db: DbPool,
    pub redis: Pool,
    pub user_service: UserService,
    pub channel_service: ChannelService,
}

pub async fn startup(database: DbPool, redis: Pool, listener: TcpListener) -> std::io::Result<()> {
    let user_service = UserService::new(database.clone(), redis.clone());
    let channel_service = ChannelService::new(database.clone(), redis.clone());

    let app_state = AppState {
        db: database,
        redis,
        user_service,
        channel_service,
    };

    let governor_conf = build_rate_limiting_conf();
    let app_state = Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .app_data(app_state.clone())
            .service(routes::ping)
            .service(
                web::scope("/api/v1")
                    .wrap(Governor::new(&governor_conf))
                    .configure(routes::configure),
            )
            .service(actix_files::Files::new("/", "./static/").index_file("index.html"))
    })
    .listen(listener)?
    .run()
    .await
}
