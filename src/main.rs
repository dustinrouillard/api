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
  influxdb::InfluxManager, prometheus::PrometheusClient,
  rabbit::RabbitManager, s3::S3Manager, valkey::ValkeyManager,
};
use envconfig::Envconfig;
use futures::future;
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
  pub prometheus: PrometheusClient,
  pub influxdb: InfluxManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let config = Config::init_from_env().unwrap();

  let env_filter =
    EnvFilter::try_from_env("DSTN_LOG").unwrap_or(EnvFilter::new("info"));
  let fmt_layer = fmt::layer().with_target(false);
  let subscriber = Registry::default().with(env_filter).with(fmt_layer);
  tracing::subscriber::set_global_default(subscriber)
    .expect("Failed to initalize global tracing subscriber");

  let valkey = connectivity::valkey::ValkeyManager::new().await;
  let rabbit = connectivity::rabbit::RabbitManager::new().await;
  let _analytics = connectivity::metrics::ApiMetrics::new().await.unwrap();

  let s3 = connectivity::s3::S3Manager::new().await.unwrap();
  let prometheus = connectivity::prometheus::PrometheusClient::new()
    .await
    .unwrap();

  let influxdb = connectivity::influxdb::InfluxManager::new().await;

  let prisma = PrismaClient::_builder().build().await.unwrap();

  if config.env == "dev" {
    tracing::info!("Running in DEV mode");
  }

  let data = web::Data::new(ServerState {
    valkey,
    rabbit,
    prisma,
    s3,
    prometheus,
    influxdb,
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
  } else {
    tracing::debug!("Spotify runner skipped due to being in DEV Mode")
  }

  let api_server = HttpServer::new(move || {
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
      .wrap(connectivity::middleware::ResponseMeta)
      .default_service(web::to(services::base::index))
      .service(services::base::health)
      .service(
        web::scope("/v2")
          .service(services::base::health)
          .service(services::analytics::factory::analytics_factory())
          .service(services::weather::factory::weather_factory())
          .service(services::hooks::factory::hooks_factory())
          .service(services::boosted::factory::boosted_factory())
          .service(services::uploads::factory::uploads_factory())
          .service(services::spotify::factory::spotify_factory())
          .service(services::github::factory::github_factory())
          .service(services::blog::factory::blog_factory()),
      )
  })
  .bind(((config.listen_host).to_owned(), config.listen_port))?
  .run();

  tracing::info!(
    "Started API Server on {}:{}",
    config.listen_host,
    config.listen_port
  );

  let metrics_server = HttpServer::new(move || {
    App::new()
      .default_service(web::to(services::base::index))
      .service(services::base::get_metrics)
      .service(services::base::health)
  })
  .bind(((config.listen_host).to_owned(), config.metrics_listen_port))?
  .run();

  tracing::info!(
    "Started metrics server on {}:{}",
    config.listen_host,
    config.metrics_listen_port
  );

  future::try_join(api_server, metrics_server).await?;

  Ok(())
}
