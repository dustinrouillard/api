use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::services::riderr::structs::{
  CurrentRideStats, RideStats, Stats,
};

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct RiderrRideUpdate {
  pub current_ride: Option<CurrentRideStats>,
  pub latest_ride: RideStats,
  pub stats: Stats,
}
