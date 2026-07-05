use std::{io::Error, sync::Arc};

use chrono::{DateTime, Utc};
use envconfig::Envconfig;
use redis::{aio::ConnectionManager, AsyncCommands};
use reqwest::Client;
use serde_json::json;
use sqlx::PgPool;

use crate::{
  config::Config,
  connectivity::valkey::ValkeyManager,
  structs::spotify::{
    AuthorizationData, CurrentPlaying, PlayerState, SpotifyAccount,
    SpotifyArtist, SpotifyDevice, SpotifyTokens,
  },
};

pub async fn set_not_playing(valkey: &mut ValkeyManager) {
  redis::cmd("SET")
    .arg("spotify/current")
    .arg(json!({"playing": false}).to_string())
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await
    .unwrap();
}

pub async fn get_playing(valkey: &mut ValkeyManager) -> CurrentPlaying {
  let redis_current_call = redis::cmd("GET")
    .arg("spotify/current")
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await
    .unwrap();

  serde_json::from_str(&redis_current_call).unwrap()
}

pub async fn update_current(
  valkey: &mut ValkeyManager,
  data: &CurrentPlaying,
) {
  redis::cmd("SET")
    .arg("spotify/current")
    .arg(serde_json::to_string(data).unwrap())
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await
    .unwrap();
}

pub async fn get_player_state(
  valkey: &mut ValkeyManager,
  alt: Option<bool>,
) -> Result<PlayerState, Error> {
  let account = get_spotify_account(valkey, alt).await?;
  let client = Client::new();

  let res = client
    .get("https://api.spotify.com/v1/me/player?additional_types=episode")
    .header("Authorization", format!("Bearer {}", account.access_token))
    .send()
    .await
    .unwrap();

  if res.status() != 200 {
    return Err(Error::new(
      std::io::ErrorKind::InvalidInput,
      "spotify player state not found",
    ));
  }

  let json = res.json::<PlayerState>().await.unwrap();

  Ok(json)
}

pub async fn get_spotify_account(
  valkey: &mut ValkeyManager,
  alt: Option<bool>,
) -> Result<SpotifyAccount, Error> {
  let mut key_base = "spotify";
  if alt.is_some() {
    key_base = "spotify_alt";
  }
  let access_token =
    valkey.cm.get(format!("{}/access_token", key_base)).await;
  let refresh_token = valkey
    .cm
    .get(format!("{}/refresh_token", key_base))
    .await
    .unwrap();

  if refresh_token == None {
    return Err(Error::new(
      std::io::ErrorKind::InvalidInput,
      "refresh_token missing",
    ));
  }

  match access_token {
    Ok(access_token) => {
      return Ok(SpotifyAccount {
        access_token,
        refresh_token,
      });
    }
    Err(..) => {
      let config = Config::init_from_env().unwrap();

      let mut extra = "";
      if alt.is_some() {
        extra = "?alt=true";
      }
      let redirect_uri =
        format!("{}{}", config.spotify_redirect_uri, extra);
      let data = AuthorizationData {
        refresh_token: refresh_token.unwrap().into(),
        grant_type: "refresh_token".into(),
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
        .await
        .unwrap();

      let status = res.status();
      if status.as_u16() == 200 {
        let body = res.json::<SpotifyTokens>().await.unwrap();

        save_spotify_tokens(
          valkey,
          &body.access_token,
          &body.refresh_token,
          &body.expires_in,
          alt,
        )
        .await;

        return Ok(SpotifyAccount {
          access_token: body.access_token,
          refresh_token: body.refresh_token,
        });
      } else {
        tracing::debug!("Error regenerating spotify tokens");
        return Ok(SpotifyAccount {
          access_token: "".into(),
          refresh_token: Some("".to_string()),
        });
      }
    }
  }
}

pub async fn save_spotify_tokens(
  valkey: &mut ValkeyManager,
  access_token: &String,
  refresh_token: &Option<String>,
  expiry_ttl: &u32,
  alt: Option<bool>,
) {
  let mut key_base = "spotify";
  if alt.is_some_and(|b| b) {
    key_base = "spotify_alt";
  }
  redis::cmd("SET")
    .arg(format!("{}/access_token", key_base))
    .arg(access_token)
    .arg("EX")
    .arg(expiry_ttl)
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await
    .ok();

  match refresh_token {
    Some(refresh_token) => {
      redis::cmd("SET")
        .arg(format!("{}/refresh_token", key_base))
        .arg(refresh_token)
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await
        .ok();
    }
    None => (),
  }
}

pub async fn get_or_make_device(
  db: &PgPool,
  name: String,
  device_type: String,
) -> SpotifyDevice {
  let name = Some(name);
  match sqlx::query_as::<_, SpotifyDevice>(
    "SELECT * FROM spotify_devices WHERE name = $1 LIMIT 1",
  )
  .bind(name.clone())
  .fetch_optional(db)
  .await
  {
    Ok(Some(device)) => device,
    _ => sqlx::query_as::<_, SpotifyDevice>(
      "INSERT INTO spotify_devices (name, type) VALUES ($1, $2) \
       RETURNING *",
    )
    .bind(name)
    .bind(Some(device_type))
    .fetch_one(db)
    .await
    .unwrap(),
  }
}

pub async fn store_history(
  db: &PgPool,
  current_playing: Arc<CurrentPlaying>,
) {
  let date: DateTime<Utc> = Utc::now();

  let dev = get_or_make_device(
    db,
    current_playing.device.as_ref().unwrap().name.to_string(),
    current_playing
      .device
      .as_ref()
      .unwrap()
      .device_type
      .to_string(),
  )
  .await;

  let mut artists: Vec<serde_json::Value> = Vec::new();
  for artist in current_playing.artists.as_ref().unwrap() {
    artists.push(json!(SpotifyArtist {
      name: artist.name.clone(),
    }));
  }

  let current_playing = current_playing.as_ref().clone();

  let type_str = current_playing
    .current_playing_type
    .as_ref()
    .unwrap()
    .to_string();

  // `alt` falls back to the column default (false) when not provided.
  sqlx::query(
    "INSERT INTO spotify_history \
       (id, type, name, length, image, device, artists, listened_at, alt) \
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, COALESCE($9, false))",
  )
  .bind(current_playing.id.unwrap())
  .bind(type_str)
  .bind(current_playing.name.unwrap())
  .bind(current_playing.length.unwrap() as i32)
  .bind(current_playing.image.unwrap())
  .bind(dev.id)
  .bind(artists)
  .bind(date)
  .bind(current_playing.alt)
  .execute(db)
  .await
  .ok();
}
