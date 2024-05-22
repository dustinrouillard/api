use std::sync::Arc;

use actix_web::web::{self};
use chrono::prelude::*;
use serde_json::json;

use crate::{
  connectivity::prisma::spotify_history,
  services::spotify::helpers::{self, get_or_make_device},
  structs::{
    self,
    spotify::{
      ArtistName, CurrentPlaying, DeviceRewrite, PlayerState,
      SpotifyArtist,
    },
  },
  ServerState,
};

extern crate chrono;
extern crate serde_json;

pub(crate) async fn fetch_spotify_current(data: web::Data<ServerState>) {
  let valkey = &mut data.valkey.clone();
  let rabbit = &mut data.rabbit.clone();

  let prisma = &mut &data.prisma;

  match helpers::get_spotify_account(valkey).await {
    Ok(account) => {
      let client = reqwest::Client::new();
      let res = client
        .get(format!(
          "https://api.spotify.com/v1/me/player?additional_types=episode"
        ))
        .header(
          "Authorization",
          format!("Bearer {}", account.access_token),
        )
        .send()
        .await
        .unwrap();

      if res.status() != 200 {
        return helpers::set_not_playing(valkey).await;
      }

      let json = res.json::<PlayerState>().await.unwrap();

      fn get_name(
        structs::spotify::Artist { name, .. }: structs::spotify::Artist,
      ) -> ArtistName {
        ArtistName { name }
      }

      if json.is_playing {
        let mut image: String = String::from("none");
        let mut artists: Vec<ArtistName> = [].to_vec();

        if json.item.album.is_some() && json.item.artists.is_some() {
          image = json.item.album.unwrap().images[0].url.to_string();
          artists = json
            .item
            .artists
            .unwrap()
            .into_iter()
            .map(get_name)
            .collect();
        } else if json.item.show.is_some() {
          let show = json.item.show.unwrap();
          image = show.images[0].url.to_string();
          artists = [ArtistName { name: show.name }].to_vec()
        }

        let current = CurrentPlaying {
          id: Some(json.item.id),
          name: Some(json.item.name),
          current_playing_type: Some(
            json.item.item_type.unwrap_or_else(|| String::from("track")),
          ),
          playing: json.is_playing,
          artists: Some(artists),
          length: Some(json.item.duration_ms),
          progress: Some(json.progress_ms),
          image: Some(image),
          device: Some(DeviceRewrite {
            name: json.device.name,
            device_type: json
              .device
              .device_type
              .unwrap_or_else(|| String::from("unknown")),
          }),
        };

        let current_query = helpers::get_playing(valkey).await;

        if current_query != current {
          let current_clone = Arc::new(current);

          helpers::update_current(valkey, &current_clone).await;
          rabbit
            .publish_spotify_current(&Arc::clone(&current_clone))
            .await;

          let date: DateTime<FixedOffset> =
            Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
          let current_playing = Arc::clone(&current_clone);

          match prisma
            .spotify_history()
            .find_first(vec![spotify_history::id::equals(
              current_playing.id.as_ref().unwrap().to_string(),
            )])
            .exec()
            .await
          {
            Ok(latest) => match latest {
              Some(latest) => {
                let listened_date = latest.listened_at.timestamp() * 1000;
                let date_minus_length =
                  (date.timestamp() * 1000) - latest.length as i64;

                if date_minus_length >= listened_date
                  && current_playing.progress.unwrap() >= 10000
                {
                  let dev = get_or_make_device(
                    prisma,
                    current_playing
                      .device
                      .as_ref()
                      .unwrap()
                      .name
                      .to_string(),
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
                  let _ = prisma.spotify_history().create(
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
                  ).exec().await;
                }
              }
              None => {
                if current_playing.progress.unwrap() >= 10000 {
                  let device = get_or_make_device(
                    prisma,
                    current_playing
                      .device
                      .as_ref()
                      .unwrap()
                      .name
                      .to_string(),
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
                  let _ = prisma.spotify_history().create(
                    current_playing.id.unwrap(),
                    current_playing.name.unwrap(),
                    current_playing.length.unwrap() as i32,
                    current_playing.image.unwrap(),
                    crate::connectivity::prisma::spotify_devices::UniqueWhereParam::IdEquals(device.id),
                    vec![
                      spotify_history::r#type::set(current_playing.current_playing_type.as_ref().unwrap().to_string()),
                      spotify_history::artists::set(artists),
                      spotify_history::listened_at::set(date),
                    ]
                  ).exec().await;
                }
              }
            },
            Err(_) => {
              if current_playing.progress.unwrap() >= 10000 {
                let device = get_or_make_device(
                  prisma,
                  current_playing
                    .device
                    .as_ref()
                    .unwrap()
                    .name
                    .to_string(),
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
                let _ = prisma.spotify_history().create(
                    current_playing.id.unwrap(),
                    current_playing.name.unwrap(),
                    current_playing.length.unwrap() as i32,
                    current_playing.image.unwrap(),
                    crate::connectivity::prisma::spotify_devices::UniqueWhereParam::IdEquals(device.id),
                    vec![
                      spotify_history::r#type::set(current_playing.current_playing_type.as_ref().unwrap().to_string()),
                      spotify_history::artists::set(artists),
                      spotify_history::listened_at::set(date),
                    ]
                  ).exec().await;
              }
            }
          }
        }
      } else {
        let current_query = helpers::get_playing(valkey).await;

        if current_query.playing {
          helpers::set_not_playing(valkey).await;
          rabbit.publish_spotify_not_playing().await;
        }
      }
    }
    Err(..) => (),
  };
}
