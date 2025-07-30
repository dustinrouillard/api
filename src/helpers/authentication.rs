use actix_web::http::header::HeaderValue;
use redis::aio::ConnectionManager;

use crate::connectivity::valkey::ValkeyManager;

pub async fn is_management_authed(
  valkey: &mut ValkeyManager,
  token: Option<&HeaderValue>,
) -> Result<bool, ()> {
  if token.is_none() {
    return Ok(false);
  }

  let token = token.unwrap().to_str().unwrap().to_string();

  let valkey_session = redis::cmd("GET")
    .arg(format!("mgmt_token/{}", token))
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await;

  Ok(valkey_session.is_ok())
}
