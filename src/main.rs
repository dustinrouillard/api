pub mod config;
pub mod connectivity;
pub mod modules;
pub mod services;
pub mod structs;

use std::{env, error::Error, time::Duration};

use actix_web::{web, App, HttpServer};
use connectivity::{postgres::PostgresManager, rabbit::RabbitManager, redis::RedisManager};
use envconfig::Envconfig;
use tokio::{sync::Mutex, time};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{fmt, prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Registry};

use crate::config::Config;

pub struct ServerState {
    pub redis: RedisManager,
    pub rabbit: RabbitManager,
    pub postgres: PostgresManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));
    let fmt_layer = fmt::layer().with_target(false);
    let subscriber = Registry::default().with(env_filter).with(fmt_layer);
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to initalize global tracing subscriber");

    let redis = connectivity::redis::RedisManager::new().await;
    let rabbit = connectivity::rabbit::RabbitManager::new().await;
    let postgres = connectivity::postgres::PostgresManager::new().await;
    let config = Config::init_from_env().unwrap();

    let addr = env::var("LISTEN_ADDR")
        .ok()
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let port = env::var("LISTEN_PORT")
        .ok()
        .unwrap_or_else(|| 8080.to_string())
        .parse::<u16>()
        .unwrap();

    tracing::info!("Starting HTTP Server on {}:{}", addr, port);

    let data = web::Data::new(Mutex::new(ServerState {
        redis,
        rabbit,
        postgres,
    }));
    let data_http = web::Data::clone(&data);

    let mut interval = time::interval(Duration::from_secs(1));

    tokio::spawn(async move {
        interval.tick().await;
        loop {
            interval.tick().await;
            tokio::spawn(modules::spotify::fetch_spotify_current(web::Data::clone(
                &data,
            )));
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::clone(&data_http))
            .wrap(TracingLogger::default())
            .service(services::base::index)
            .service(services::spotify::current)
            .service(services::spotify::authorize)
            .service(services::spotify::setup)
    })
    .bind(((config.listen_host).to_owned(), config.listen_port))?
    .run()
    .await
    .unwrap();

    Ok(())
}
