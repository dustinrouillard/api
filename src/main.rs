pub mod config;
pub mod connectivity;
pub mod modules;
pub mod services;
pub mod structs;

use std::{error::Error, time::Duration};

use actix_web::{middleware, web, App, HttpServer};
use actix_web_lab::middleware::from_fn;
use connectivity::{rabbit::RabbitManager, valkey::ValkeyManager};
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
    App::new()
      .app_data(web::Data::clone(&data_http))
      .wrap(middleware::NormalizePath::default())
      .wrap(TracingLogger::default())
      .default_service(web::to(services::base::index))
      .service(
        web::scope("/v2")
          .service(services::base::health)
          .service(
            web::scope("/spotify")
              .service(services::spotify::routes::current)
              .service(services::spotify::routes::authorize)
              .service(services::spotify::routes::setup),
          )
          .service(
            web::scope("/blog")
              .service(services::blog::auth::login)
              .service(services::blog::posts::get_post)
              .service(services::blog::posts::get_posts)
              .service(
                web::scope("")
                  .wrap(from_fn(
                    services::blog::middleware::blog_admin_auth_mw,
                  ))
                  .service(services::blog::auth::logout)
                  .service(services::blog::auth::get_user)
                  .service(services::blog::posts::create_post)
                  .service(services::blog::posts::update_post)
                  .service(services::blog::posts::delete_post),
              ),
          ),
      )
  })
  .bind(((config.listen_host).to_owned(), config.listen_port))?
  .run()
  .await
  .unwrap();

  Ok(())
}
