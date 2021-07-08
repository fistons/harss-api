#[macro_use]
extern crate diesel;

use actix_files as fs;
use actix_web::{web, App, HttpServer};
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

use crate::model::configuration::ApplicationConfiguration;
use crate::services::channels::ChannelService;
use crate::services::items::ItemService;
use crate::services::users::UserService;
use crate::services::GlobalService;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;
use ttl_cache::TtlCache;

mod errors;
mod model;
mod routes;
mod schema;
mod services;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenv::dotenv().ok();

    // Init Logger
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Always,
    )])
    .unwrap();

    // set up database connection pool
    let connection_spec = std::env::var("DATABASE_URL").unwrap_or_else(|_| String::from("rss.db"));
    let manager = ConnectionManager::<PgConnection>::new(connection_spec);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let item_service = ItemService::new(pool.clone());
    let channel_service = ChannelService::new(pool.clone());
    let user_service = UserService::new(pool.clone());
    let global_service = GlobalService::new(item_service.clone(), channel_service.clone());

    let configuration = load_configuration().unwrap();

    let cache = web::Data::new(Cache::new());

    HttpServer::new(move || {
        App::new()
            .data(global_service.clone())
            .data(item_service.clone())
            .data(channel_service.clone())
            .data(user_service.clone())
            .data(configuration.clone())
            .app_data(cache.clone())
            .configure(routes::channels::configure)
            .configure(routes::service::configure)
            .configure(routes::users::configure)
            .configure(routes::auth::configure)
            .service(fs::Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(std::env::var("LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")))?
    .run()
    .await
}

fn load_configuration() -> Result<ApplicationConfiguration, Box<dyn Error>> {
    let file = File::open(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("configuration.yaml")),
    )?;
    let reader = BufReader::new(file);
    let configuration = serde_yaml::from_reader(reader)?;

    Ok(configuration)
}

pub struct Cache {
    pub cache_mutex: Mutex<TtlCache<String, String>>,
}

impl Cache {
    fn new() -> Self {
        Cache {
            cache_mutex: Mutex::new(TtlCache::new(1024)),
        }
    }
}
