use actix_web::{get, http::Error, web, HttpResponse};
use chrono::{DateTime, Utc};
use envconfig::Envconfig;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;

use crate::{
  config::Config,
  services::spotify::helpers,
  structs::spotify::{
    AuthorizationData, RecentSongQuery, SpotifyQueryString, SpotifyTokens,
  },
  ServerState,
};

/// Flattened `spotify_history` row joined with its device's name/type.
#[derive(Debug, FromRow)]
struct RecentRow {
  id: String,
  r#type: String,
  name: String,
  artists: Vec<serde_json::Value>,
  length: i32,
  image: String,
  listened_at: DateTime<Utc>,
  alt: Option<bool>,
  device_name: Option<String>,
  device_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthorizeQuery {
  code: String,
  alt: Option<bool>,
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

  Ok(HttpResponse::Ok().json(json!({"success": true, "data": &json})))
}

#[get("/recents")]
async fn recent_listens(
  state: web::Data<ServerState>,
  query: Option<web::Query<RecentSongQuery>>,
) -> Result<HttpResponse, Error> {
  let query: web::Query<RecentSongQuery> = query
    .unwrap_or(actix_web::web::Query(RecentSongQuery { limit: Some(10) }));

  let recents = sqlx::query_as::<_, RecentRow>(
    "SELECT h.id, h.type, h.name, h.artists, h.length, h.image, \
       h.listened_at, h.alt, d.name AS device_name, d.type AS device_type \
     FROM spotify_history h \
     LEFT JOIN spotify_devices d ON d.id = h.device \
     ORDER BY h.listened_at DESC LIMIT $1",
  )
  .bind(query.limit.unwrap_or(10))
  .fetch_all(&state.db)
  .await;

  let recents: Vec<serde_json::Value> = recents
    .unwrap()
    .into_iter()
    .map(|recent| {
      json!({
        "id": recent.id,
        "type": recent.r#type,
        "name": recent.name,
        "artists": recent.artists,
        "length": recent.length,
        "image": recent.image,
        "device": {
          "name": recent.device_name,
          "type": recent.device_type,
        },
        "listened_at": recent.listened_at,
        "alt": recent.alt,
      })
    })
    .collect();

  Ok(HttpResponse::Ok().json(json!({"recents": recents})))
}

#[get("/authorize")]
async fn authorize(
  data: web::Data<ServerState>,
  query: web::Query<SpotifyQueryString>,
) -> Result<HttpResponse, Error> {
  let valkey = &mut data.valkey.clone();

  let mut redirect_extra = "";
  let mut base_key = "spotify";
  let alt = query.alt.unwrap_or(false);
  if alt {
    base_key = "spotify_alt";
    redirect_extra = "?alt=true";
  }

  let setup_check = valkey
    .cm
    .exists(format!("{}/refresh_token", base_key))
    .await
    .unwrap_or(false);
  if setup_check {
    tracing::debug!("spotify already setup");
    let json = json!({
        "code": "already_authorized",
        "message": "Spotify is already setup."
    });

    return Ok(HttpResponse::BadRequest().json(json));
  }

  let config = Config::init_from_env().unwrap();

  let scope = "user-read-playback-state+user-read-currently-playing";
  let redirect_uri =
    format!("{}{}", config.spotify_redirect_uri, redirect_extra);
  let url = format!("https://accounts.spotify.com/authorize?client_id={}&response_type=code&scope={}&redirect_uri={}", config.spotify_client_id, scope, redirect_uri);
  let json = json!({ "url": url });

  Ok(HttpResponse::Ok().json(json))
}

#[get("/setup")]
async fn setup(
  data: web::Data<ServerState>,
  info: web::Query<AuthorizeQuery>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
  let valkey = &mut data.valkey.clone();

  let mut redirect_extra = "";
  let mut base_key = "spotify";
  let alt = info.alt.unwrap_or(false);
  if alt {
    base_key = "spotify_alt";
    redirect_extra = "?alt=true";
  }

  let setup_check = valkey
    .cm
    .exists(format!("{}/refresh_token", base_key))
    .await
    .unwrap_or(false);
  if setup_check {
    tracing::debug!("spotify already setup");
    let json = json!({
        "code": "already_authorized",
        "message": "Spotify is already setup."
    });

    return Ok(HttpResponse::BadRequest().json(json));
  }

  let config = Config::init_from_env().unwrap();

  let code = &info.code;
  let redirect_uri =
    format!("{}{}", config.spotify_redirect_uri, redirect_extra);
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
      .arg(format!("{}/access_token", base_key))
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
          .arg(format!("{}/refresh_token", base_key))
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
