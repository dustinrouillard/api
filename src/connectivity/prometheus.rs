use std::io::Error;

use prometheus_http_query::Client;

use envconfig::Envconfig;

use crate::config::Config;

#[warn(dead_code)]
#[derive(Clone)]
pub struct PrometheusClient {
  pub client: Client,
}

impl PrometheusClient {
  pub async fn new() -> Result<Self, Error> {
    let config = Config::init_from_env().unwrap();

    let client = Client::try_from(config.prometheus_host).unwrap();

    Ok(Self { client })
  }
}
