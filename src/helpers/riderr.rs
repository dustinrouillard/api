use envconfig::Envconfig;

use crate::{
  config::Config, connectivity::rabbit::RabbitManager,
  services::riderr::structs::RiderrUserStats,
  structs::riderr::RiderrRideUpdate,
};

pub async fn send_riderr_event(rabbit: &mut RabbitManager) {
  let config = Config::init_from_env().unwrap();

  let client = reqwest::Client::new();
  let res = client
    .get(format!("{}/v1/users/stats", config.riderr_api_endpoint))
    .header("Authorization", config.riderr_api_token)
    .send()
    .await
    .unwrap();

  let json = res.json::<RiderrUserStats>().await.unwrap();

  let state = RiderrRideUpdate {
    current_ride: json.current_ride,
    latest_ride: json.latest_ride,
    stats: json.stats,
  };

  rabbit.publish_ride_state(&state).await;
}
