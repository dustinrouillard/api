use envconfig::Envconfig;
use influxdb2::Client;

use crate::config::Config;

#[derive(Clone)]
pub struct InfluxManager {
  pub client: Client,
}

impl InfluxManager {
  pub async fn new() -> Self {
    let config = Config::init_from_env().unwrap();
    let client = influxdb2::Client::new(
      config.influxdb_host,
      config.influxdb_org,
      config.influxdb_token,
    );

    tracing::info!("Connected to influxdb");
    Self { client }
  }
}
