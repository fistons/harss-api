use std::net::TcpListener;

use actix_governor::Governor;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use deadpool_redis::Pool;
use sea_orm::DatabaseConnection;

use crate::rate_limiting::build_rate_limiting_conf;
use crate::routes;

pub struct AppState {
    pub db: DatabaseConnection,
}

pub async fn startup(
    database: DatabaseConnection,
    redis: Pool,
    listener: TcpListener,
) -> std::io::Result<()> {
    let application_service = AppState { db: database };

    let governor_conf = build_rate_limiting_conf();
    let services = Data::new(application_service);
    let redis = Data::new(redis);

    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(sentry_actix::Sentry::default())
            .app_data(services.clone())
            .app_data(redis.clone())
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
