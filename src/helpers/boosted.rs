use envconfig::Envconfig;
use redis::aio::ConnectionManager;

use crate::{
  config::Config,
  connectivity::{rabbit::RabbitManager, valkey::ValkeyManager},
  services::boosted::structs::BoostedStats,
  structs::boosted::BoostedRideUpdate,
};

pub async fn send_boosted_event(
  valkey: &mut ValkeyManager,
  rabbit: &mut RabbitManager,
) {
  let config = Config::init_from_env().unwrap();

  let in_ride = redis::cmd("GET")
    .arg("boosted/in-ride")
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await
    .unwrap_or(String::from("false"))
    == "true";

  let client = reqwest::Client::new();
  let res = client
    .get(format!("{}/v1/users/stats", config.boosted_api_endpoint))
    .header("Authorization", config.boosted_api_token)
    .send()
    .await
    .unwrap();

  let json = res.json::<BoostedStats>().await.unwrap();

  let state = BoostedRideUpdate {
    riding: in_ride,
    current_ride: json.current_ride,
    latest_ride: json.latest_ride,
    stats: json.stats,
  };

  rabbit.publish_ride_state(&state).await;
}
