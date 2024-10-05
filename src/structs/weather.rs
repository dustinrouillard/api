use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UpstreamWeather {
  pub city: String,
  pub temperature: Temperature,
  pub humidity: i64,
  pub conditions: Vec<Condition>,
}

#[derive(Serialize, Deserialize)]
pub struct Condition {
  pub icon: Option<String>,
  pub code: String,
  pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct Temperature {
  pub current: f64,
  pub max: f64,
  pub min: f64,
}
