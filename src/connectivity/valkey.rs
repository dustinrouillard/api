use envconfig::Envconfig;
use redis::{aio::ConnectionManager, Client};

use crate::config::Config;

#[derive(Clone)]
pub struct ValkeyManager {
  pub cm: ConnectionManager,
}

impl ValkeyManager {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let client = Client::open(config.valkey_dsn).unwrap();
    let cm = ConnectionManager::new(client).await.unwrap();
    tracing::info!("Connected to valkey");
    Self { cm }
  }
}
