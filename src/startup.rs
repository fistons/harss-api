use std::net::TcpListener;

use actix_web::web::Data;
use actix_web::{App, HttpServer};
use deadpool_redis::Pool;
use sea_orm::DatabaseConnection;

use crate::model::configuration::ApplicationConfiguration;
use crate::routes;
use crate::services::channels::ChannelService;
use crate::services::items::ItemService;
use crate::services::users::UserService;
use crate::services::GlobalService;

#[derive(Clone)]
pub struct ApplicationServices {
    pub global_service: GlobalService,
    pub item_service: ItemService,
    pub channel_service: ChannelService,
    pub user_service: UserService,
}

fn build_services(database: &DatabaseConnection) -> ApplicationServices {
    let item_service = ItemService::new(database.clone());
    let channel_service = ChannelService::new(database.clone());
    let user_service = UserService::new(database.clone());
    let global_service = GlobalService::new(item_service.clone(), channel_service.clone());

    ApplicationServices {
        global_service,
        item_service,
        channel_service,
        user_service,
    }
}

pub async fn startup(
    database: DatabaseConnection,
    redis: Pool,
    configuration: ApplicationConfiguration,
    listener: TcpListener,
) -> std::io::Result<()> {
    let application_service = build_services(&database);

    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .app_data(Data::new(application_service.clone()))
            .app_data(Data::new(configuration.clone()))
            .app_data(Data::new(redis.clone()))
            .configure(routes::configure)
            .service(actix_files::Files::new("/", "./static/").index_file("index.html"))
    })
    .listen(listener)?
    .run()
    .await
}
