use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::services::boosted::structs::{
  CurrentRideStats, RideStats, Stats,
};

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct BoostedRideUpdate {
  pub riding: bool,
  pub current_ride: Option<CurrentRideStats>,
  pub latest_ride: RideStats,
  pub stats: Stats,
}
