use actix_web::{http::Error, post, web, HttpRequest, HttpResponse};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use envconfig::Envconfig as _;
use hmac::{Hmac, Mac};
use redis::aio::ConnectionManager;
use sha2::Sha256;

use crate::{
  config::Config,
  helpers::riderr::send_riderr_event,
  services::hooks::structs::{RiderrHookPayload, RiderrHookType},
  ServerState,
};

#[post("/riderr")]
async fn execute(
  req: HttpRequest,
  state: web::Data<ServerState>,
  payload: web::Json<RiderrHookPayload>,
) -> Result<HttpResponse, Error> {
  let config = Config::init_from_env().unwrap();

  let auth_header = req.headers().get("x-hook-signature");
  if auth_header.is_none() {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let received_signature = auth_header.unwrap().to_str().unwrap();
  let received_signature_bytes = match STANDARD.decode(received_signature)
  {
    Ok(bytes) => bytes,
    Err(_) => {
      return Ok(
        HttpResponse::BadRequest().body("Invalid signature encoding"),
      )
    }
  };

  let mut mac =
    Hmac::<Sha256>::new_from_slice(config.riderr_hook_token.as_bytes())
      .expect("HMAC can take key of any size");
  let payload_string =
    serde_json::to_string::<RiderrHookPayload>(&payload)
      .unwrap_or("".into());
  mac.update(payload_string.as_bytes());

  let expected_signature = mac.finalize().into_bytes().to_vec();
  if expected_signature != received_signature_bytes {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let valkey = &mut state.valkey.clone();
  let rabbit = &mut state.rabbit.clone();

  match payload.hook_type {
    RiderrHookType::RideStarted => {
      let _ = redis::cmd("SET")
        .arg("riderr/in-ride")
        .arg("true")
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      send_riderr_event(rabbit).await;
    }
    RiderrHookType::RideEnded => {
      let _ = redis::cmd("DEL")
        .arg("riderr/in-ride")
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      send_riderr_event(rabbit).await;
    }
    RiderrHookType::RideDiscarded => {
      let _ = redis::cmd("DEL")
        .arg("riderr/in-ride")
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      send_riderr_event(rabbit).await;
    }
    RiderrHookType::BoardUpdated => {
      send_riderr_event(rabbit).await;
    }
  }

  Ok(HttpResponse::NoContent().finish())
}
