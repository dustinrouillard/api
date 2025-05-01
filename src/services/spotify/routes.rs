use actix_web::{get, http::Error, web, HttpResponse};
use envconfig::Envconfig;
use prisma_client_rust::Direction;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
  config::Config,
  connectivity::prisma::spotify_history,
  services::spotify::helpers,
  structs::spotify::{
    AuthorizationData, RecentSongQuery, SpotifyQueryString, SpotifyTokens,
  },
  ServerState,
};

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
  let prisma = &mut &state.prisma;

  let query: web::Query<RecentSongQuery> = query
    .unwrap_or(actix_web::web::Query(RecentSongQuery { limit: Some(10) }));

  let recents = prisma
    .spotify_history()
    .find_many(vec![])
    .order_by(spotify_history::listened_at::order(Direction::Desc))
    .take(query.limit.unwrap_or(10))
    .include(spotify_history::include!({ spotify_devices: select { name r#type } }))
    .exec()
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
        "device": recent.spotify_devices,
        "listened_at": recent.listened_at
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
