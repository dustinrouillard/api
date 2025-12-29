use actix_web::{get, http::Error, web, HttpResponse};
use envconfig::Envconfig;
use redis::aio::ConnectionManager;
use serde_json::json;

use crate::{
  config::Config, services::riderr::structs::RiderrUserStats, ServerState,
};

#[get("/stats")]
async fn ride_stats(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let config = Config::init_from_env().unwrap();
  let valkey = &mut state.valkey.clone();

  let in_ride = redis::cmd("GET")
    .arg("riderr/in-ride")
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await
    .unwrap_or(String::from("false"))
    == "true";

  let client = reqwest::Client::new();
  let res = client
    .get(format!("{}/v1/users/stats", config.riderr_api_endpoint))
    .header("Authorization", config.riderr_api_token)
    .send()
    .await
    .unwrap();

  let json = res.json::<RiderrUserStats>().await;

  if json.is_err() {
    return Ok(HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch stats"})));
  }

  let json = json.unwrap();

  Ok(HttpResponse::Ok().json(json!({"riderr": {
    "riding": in_ride,
    "current_ride": json.current_ride,
    "latest_ride": json.latest_ride,
    "stats": json.stats
  }})))
}
