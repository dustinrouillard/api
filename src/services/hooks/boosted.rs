use actix_web::{http::Error, post, web, HttpRequest, HttpResponse};
use envconfig::Envconfig as _;
use redis::aio::ConnectionManager;

use crate::{
  config::Config,
  helpers::boosted::send_boosted_event,
  services::hooks::structs::{BoostedHookPayload, BoostedHookType},
  ServerState,
};

#[post("/boosted")]
async fn execute(
  req: HttpRequest,
  state: web::Data<ServerState>,
  payload: web::Json<BoostedHookPayload>,
) -> Result<HttpResponse, Error> {
  let config = Config::init_from_env().unwrap();

  let auth_header = req.headers().get("authorization");
  if auth_header.is_none() {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let auth_header = auth_header.unwrap().to_str().unwrap();

  if auth_header != config.boosted_hook_token {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let valkey = &mut state.valkey.clone();
  let rabbit = &mut state.rabbit.clone();

  match payload.hook_type {
    BoostedHookType::RideStarted => {
      let _ = redis::cmd("SET")
        .arg("boosted/in-ride")
        .arg("true")
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      send_boosted_event(valkey, rabbit).await;
    }
    BoostedHookType::RideEnded => {
      let _ = redis::cmd("DEL")
        .arg("boosted/in-ride")
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      send_boosted_event(valkey, rabbit).await;
    }
    BoostedHookType::RideDiscarded => {
      let _ = redis::cmd("DEL")
        .arg("boosted/in-ride")
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      send_boosted_event(valkey, rabbit).await;
    }
    BoostedHookType::BoardUpdated => {
      send_boosted_event(valkey, rabbit).await;
    }
  }

  Ok(HttpResponse::NoContent().finish())
}
