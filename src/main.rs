use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;
use std::time::Duration;

use actix_files as fs;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use clokwerk::{AsyncScheduler, TimeUnits};
use sea_orm::{ConnectOptions, Database};
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::model::configuration::ApplicationConfiguration;
use crate::services::channels::ChannelService;
use crate::services::items::ItemService;
use crate::services::users::UserService;
use crate::services::GlobalService;

mod errors;
mod model;
mod routes;
mod services;

type RedisConnection = redis::Connection;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenv::dotenv().ok();

    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("rss-aggregator")
        .install_simple()
        .unwrap();
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("rss-aggregator".into(), std::io::stdout);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .with(telemetry);
    set_global_default(subscriber).expect("Failed to set subscriber");

    // set up database connection pool
    let connection_spec = std::env::var("DATABASE_URL").unwrap_or_else(|_| String::from("rss.db"));
    let mut opt = ConnectOptions::new(connection_spec.to_owned());
    opt.min_connections(5)
        .max_connections(10)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8));

    let db = Database::connect(opt)
        .await
        .expect("Could not connect to postgres");

    let item_service = ItemService::new(db.clone());
    let channel_service = ChannelService::new(db.clone());
    let user_service = UserService::new(db.clone());
    let global_service = GlobalService::new(item_service.clone(), channel_service.clone());

    let configuration = load_configuration().unwrap();

    let mut scheduler = AsyncScheduler::new();
    let global = global_service.clone();

    let polling = std::env::var("POLLING_INTERVAL")
        .unwrap_or_else(|_| String::from("300"))
        .parse::<u32>()
        .unwrap()
        .seconds();
    tracing::info!("Poll every {:?}", polling);

    scheduler
        .every(polling)
        .run(move || refresh(global.clone()));
    actix_rt::spawn(async move {
        loop {
            scheduler.run_pending().await;
            actix_rt::time::sleep(Duration::from_millis(100)).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .app_data(Data::new(global_service.clone()))
            .app_data(Data::new(item_service.clone()))
            .app_data(Data::new(channel_service.clone()))
            .app_data(Data::new(user_service.clone()))
            .app_data(Data::new(configuration.clone()))
            .app_data(Data::new(RefreshTokenStore::new()))
            .configure(routes::channels::configure)
            .configure(routes::users::configure)
            .configure(routes::auth::configure)
            .configure(routes::items::configure)
            .service(fs::Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(std::env::var("LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")))?
    .run()
    .await
}

async fn refresh(service: GlobalService) {
    service.refresh_all_channels().await;
}

fn load_configuration() -> Result<ApplicationConfiguration, Box<dyn Error>> {
    let file = File::open(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("configuration.yaml")),
    )?;
    let reader = BufReader::new(file);
    let configuration = serde_yaml::from_reader(reader)?;

    Ok(configuration)
}

pub struct RefreshTokenStore {
    pub store: Mutex<RedisConnection>,
}

impl RefreshTokenStore {
    fn new() -> Self {
        let connection = redis::Client::open(
            std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1")),
        )
        .unwrap()
        .get_connection()
        .unwrap();

        RefreshTokenStore {
            store: Mutex::new(connection),
        }
    }
}
