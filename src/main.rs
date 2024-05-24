pub mod config;
pub mod connectivity;
pub mod modules;
pub mod services;
pub mod structs;

use std::{error::Error, time::Duration};

use actix_cors::Cors;
use actix_multipart::form::MultipartFormConfig;
use actix_web::{middleware, web, App, HttpServer};
use connectivity::{
  rabbit::RabbitManager, s3::S3Manager, valkey::ValkeyManager,
};
use envconfig::Envconfig;
use tokio::time;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{
  fmt, prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Registry,
};

use connectivity::prisma::PrismaClient;

use crate::config::Config;

pub struct ServerState {
  pub valkey: ValkeyManager,
  pub rabbit: RabbitManager,
  pub prisma: PrismaClient,
  pub s3: S3Manager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let config = Config::init_from_env().unwrap();

  let env_filter =
    EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));
  let fmt_layer = fmt::layer().with_target(false);
  let subscriber = Registry::default().with(env_filter).with(fmt_layer);
  tracing::subscriber::set_global_default(subscriber)
    .expect("Failed to initalize global tracing subscriber");

  let valkey = connectivity::valkey::ValkeyManager::new().await;
  let rabbit = connectivity::rabbit::RabbitManager::new().await;
  let s3 = connectivity::s3::S3Manager::new().await.unwrap();

  let prisma = PrismaClient::_builder().build().await.unwrap();

  tracing::info!(
    "Starting HTTP Server on {}:{}",
    config.listen_host,
    config.listen_port
  );

  if config.env == "dev" {
    tracing::info!("Running in DEV mode");
  }

  let data = web::Data::new(ServerState {
    valkey,
    rabbit,
    prisma,
    s3,
  });
  let data_http = web::Data::clone(&data);

  // Fetch spotify current playing every second.
  let mut interval = time::interval(Duration::from_secs(1));
  if config.env != "dev" {
    tokio::spawn(async move {
      interval.tick().await;
      loop {
        interval.tick().await;
        tokio::spawn(modules::spotify::fetch_spotify_current(
          web::Data::clone(&data),
        ));
      }
    });
  }

  HttpServer::new(move || {
    let cors = Cors::default()
      .allowed_origin_fn(|origin, _req_head| {
        origin.as_bytes().ends_with(b".dstn.to")
      })
      .allowed_origin_fn(|origin, _req_head| {
        origin.as_bytes().ends_with(b":3000")
      });

    App::new()
      .app_data(web::Data::clone(&data_http))
      .app_data(
        MultipartFormConfig::default()
          .total_limit(10 * 1024 * 1024)
          .memory_limit(10 * 1024 * 1024),
      )
      .wrap(cors)
      .wrap(middleware::NormalizePath::default())
      .wrap(TracingLogger::default())
      .default_service(web::to(services::base::index))
      .service(
        web::scope("/v2")
          .service(services::base::health)
          .service(services::uploads::factory::uploads_factory())
          .service(services::spotify::factory::spotify_factory())
          .service(services::github::factory::github_factory())
          .service(services::blog::factory::blog_factory()),
      )
  })
  .bind(((config.listen_host).to_owned(), config.listen_port))?
  .run()
  .await
  .unwrap();

  Ok(())
}
