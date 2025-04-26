use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct SiteSettingsResponse {
  pub settings: HashMap<String, String>,
}
