use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum BoostedHookType {
  #[serde(rename = "ride_started")]
  RideStarted,
  #[serde(rename = "ride_ended")]
  RideEnded,
  #[serde(rename = "ride_discarded")]
  RideDiscarded,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RideSummary {
  pub ride_id: i64,
  pub distance: f64,
  pub max_speed: f64,
  pub avg_speed: f64,
  pub elevation_gain: Option<f64>,
  pub elevation_loss: Option<f64>,
  pub ride_points: usize,
  pub start_time: DateTime<FixedOffset>,
  pub end_time: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RideStartedHookBody {
  pub ride_id: i64,
  pub started_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RideEndedHookBody {
  pub ride_id: i64,
  pub summary: RideSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RideDiscardedHookBody {
  pub ride_id: i64,
  pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoostedHookPayload {
  pub hook_type: BoostedHookType,
  pub body: serde_json::Value,
}
