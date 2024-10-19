use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BoostedStats {
  pub latest_ride: RideStats,
  pub stats: Stats,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RideStats {
  pub started_at: String,
  pub ended_at: String,
  pub duration: f64,
  pub distance: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stats {
  pub boards: Vec<Board>,
  pub rides: ValueEntry,
  pub duration: ValueEntry,
  pub distance: ValueEntry,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Board {
  pub id: i64,
  pub name: String,
  pub odometer: f64,
  pub battery: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ValueEntry {
  pub day: f64,
  pub week: f64,
  pub month: f64,
}
