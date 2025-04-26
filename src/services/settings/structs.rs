use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SiteSettingsResponse {
  pub settings: HashMap<String, String>,
}
