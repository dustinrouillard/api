use std::collections::HashMap;

use actix_web::{get, http::Error, web, HttpResponse};
use redis::AsyncCommands;
use serde_json::{json, Value};

use crate::{
  services::settings::structs::SiteSettingsResponse, ServerState,
};

#[get("/site")]
pub async fn get_site_settings(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let redis = &mut state.valkey.clone();
  let settings = redis
    .cm
    .hgetall::<_, HashMap<String, String>>("settings/site")
    .await
    .unwrap_or(HashMap::new());

  Ok(HttpResponse::Ok().json(SiteSettingsResponse { settings }))
}
