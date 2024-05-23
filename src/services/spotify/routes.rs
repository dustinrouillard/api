use actix_web::{get, http::Error, web, HttpResponse};
use envconfig::Envconfig;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
  config::Config,
  services::spotify::helpers,
  structs::spotify::{AuthorizationData, SpotifyTokens},
  ServerState,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AuthorizeQuery {
  code: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenError {
  error: String,
  error_description: String,
}

#[get("")]
async fn current(
  data: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let valkey = &mut data.valkey.clone();
  let json = helpers::get_playing(valkey).await;

  Ok(
    HttpResponse::Ok()
      .insert_header(("Content-Type", "application/json"))
      .body(json!({"success": true, "data": &json}).to_string()),
  )
}

#[get("/authorize")]
async fn authorize(
  data: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let valkey = &mut data.valkey.clone();

  let setup_check = valkey
    .cm
    .exists("spotify/refresh_token")
    .await
    .unwrap_or(false);
  if setup_check {
    tracing::debug!("spotify already setup");
    let json = json!({
        "code": "already_authorized",
        "message": "Spotify is already setup."
    });

    return Ok(
      HttpResponse::BadRequest()
        .insert_header(("Content-Type", "application/json"))
        .body(json.to_string()),
    );
  }

  let config = Config::init_from_env().unwrap();

  let scope = "user-read-playback-state+user-read-currently-playing";
  let redirect_uri = "http://127.0.0.1:8080/spotify/setup";
  let url = format!("https://accounts.spotify.com/authorize?client_id={}&response_type=code&scope={}&redirect_uri={}", config.spotify_client_id, scope, redirect_uri);
  let json = json!({ "url": url });

  Ok(
    HttpResponse::Ok()
      .append_header(("Content-type", "application/json"))
      .body(json.to_string()),
  )
}

#[get("/setup")]
async fn setup(
  data: web::Data<ServerState>,
  info: web::Query<AuthorizeQuery>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
  let valkey = &mut data.valkey.clone();

  let setup_check = valkey
    .cm
    .exists("spotify/refresh_token")
    .await
    .unwrap_or(false);
  if setup_check {
    tracing::debug!("spotify already setup");
    let json = json!({
        "code": "already_authorized",
        "message": "Spotify is already setup."
    });

    return Ok(
      HttpResponse::BadRequest()
        .insert_header(("Content-Type", "application/json"))
        .body(json.to_string()),
    );
  }

  let config = Config::init_from_env().unwrap();

  let code = &info.code;
  let redirect_uri = "http://127.0.0.1:8080/spotify/setup";
  let data = AuthorizationData {
    code: code.clone().into(),
    grant_type: "authorization_code".into(),
    redirect_uri: redirect_uri.into(),
    ..AuthorizationData::default()
  };

  let data = serde_urlencoded::to_string(&data)
    .expect("error serializing data for spotify token");

  let client = reqwest::Client::new();
  let res = client
    .post(format!("https://accounts.spotify.com/api/token?{}", data))
    .header("Content-Type", "application/x-www-form-urlencoded")
    .header("Content-Length", "0")
    .basic_auth(
      config.spotify_client_id,
      Some(config.spotify_client_secret),
    )
    .send()
    .await?;

  let status = res.status();

  if status.as_u16() == 200 {
    let body = res.json::<SpotifyTokens>().await.unwrap();

    redis::cmd("SET")
      .arg("spotify/access_token")
      .arg(&body.access_token)
      .arg("EX")
      .arg(&body.expires_in)
      .query_async::<ConnectionManager, String>(&mut valkey.cm)
      .await
      .unwrap();

    let refresh_token = &body.refresh_token;
    match refresh_token {
      Some(refresh_token) => {
        redis::cmd("SET")
          .arg("spotify/refresh_token")
          .arg(refresh_token)
          .query_async::<ConnectionManager, String>(&mut valkey.cm)
          .await
          .unwrap();
      }
      None => (),
    }

    Ok(HttpResponse::NoContent().finish())
  } else {
    let body = res.json::<TokenError>().await.unwrap();
    Ok(HttpResponse::InternalServerError().json(body))
  }
}
