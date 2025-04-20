use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum RiderrHookType {
  #[serde(rename = "ride_started")]
  RideStarted,
  #[serde(rename = "ride_ended")]
  RideEnded,
  #[serde(rename = "ride_discarded")]
  RideDiscarded,
  #[serde(rename = "board_updated")]
  BoardUpdated,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RideSummary {
  pub id: String,
  pub duration: u64,
  pub distance: f64,
  pub max_speed: f64,
  pub avg_speed: f64,
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

pub struct BoardUpdatedHookBody {
  pub board_id: i64,
  pub board_name: String,
  pub board_serial: String,
  pub board_odometer: f64,
  pub board_battery: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiderrHookPayload {
  pub hook_type: RiderrHookType,
  pub body: serde_json::Value,
}
