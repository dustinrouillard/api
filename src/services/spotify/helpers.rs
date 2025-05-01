use std::{io::Error, sync::Arc};

use chrono::{DateTime, FixedOffset, Utc};
use envconfig::Envconfig;
use redis::{aio::ConnectionManager, AsyncCommands};
use reqwest::Client;
use serde_json::json;

use crate::{
  config::Config,
  connectivity::{
    prisma::{
      spotify_devices, spotify_history, spotify_history_alt, PrismaClient,
    },
    valkey::ValkeyManager,
  },
  structs::spotify::{
    AuthorizationData, CurrentPlaying, PlayerState, SpotifyAccount,
    SpotifyArtist, SpotifyTokens,
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
  prisma: &mut &PrismaClient,
  name: String,
  device_type: String,
) -> spotify_devices::Data {
  let name = Some(name);
  match prisma
    .spotify_devices()
    .find_first(vec![spotify_devices::name::equals(name.clone())])
    .exec()
    .await
  {
    Ok(device) => match device {
      Some(device) => device,
      None => prisma
        .spotify_devices()
        .create(vec![
          spotify_devices::name::set(name),
          spotify_devices::r#type::set(Some(device_type)),
        ])
        .exec()
        .await
        .unwrap(),
    },
    Err(_) => prisma
      .spotify_devices()
      .create(vec![
        spotify_devices::name::set(name),
        spotify_devices::r#type::set(Some(device_type)),
      ])
      .exec()
      .await
      .unwrap(),
  }
}

pub async fn store_history(
  prisma: &mut &PrismaClient,
  current_playing: Arc<CurrentPlaying>,
  alt: Option<bool>,
) {
  let date: DateTime<FixedOffset> =
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());

  let dev = get_or_make_device(
    prisma,
    current_playing.device.as_ref().unwrap().name.to_string(),
    current_playing
      .device
      .as_ref()
      .unwrap()
      .device_type
      .to_string(),
  )
  .await;

  let mut artists = Vec::new();
  for artist in current_playing.artists.as_ref().unwrap() {
    artists.push(json!(SpotifyArtist {
      name: artist.name.clone(),
    }));
  }

  let current_playing = current_playing.as_ref().clone();
  if alt.is_some_and(|b| b) {
    prisma.spotify_history_alt().create(
        current_playing.id.unwrap(),
        current_playing.name.unwrap(),
        current_playing.length.unwrap() as i32,
        current_playing.image.unwrap(),
        crate::connectivity::prisma::spotify_devices::UniqueWhereParam::IdEquals(dev.id),
        vec![
          spotify_history_alt::r#type::set(current_playing.current_playing_type.as_ref().unwrap().to_string()),
          spotify_history_alt::artists::set(artists),
          spotify_history_alt::listened_at::set(date),
        ]
      ).exec().await.ok();
  } else {
    prisma.spotify_history().create(
        current_playing.id.unwrap(),
        current_playing.name.unwrap(),
        current_playing.length.unwrap() as i32,
        current_playing.image.unwrap(),
        crate::connectivity::prisma::spotify_devices::UniqueWhereParam::IdEquals(dev.id),
        vec![
          spotify_history::r#type::set(current_playing.current_playing_type.as_ref().unwrap().to_string()),
          spotify_history::artists::set(artists),
          spotify_history::listened_at::set(date),
        ]
      ).exec().await.ok();
  };
}
