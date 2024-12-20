use std::sync::Arc;

use actix_web::web::{self};
use chrono::prelude::*;
use prisma_client_rust::Direction;

use crate::{
  connectivity::prisma::spotify_history,
  services::spotify::helpers::{self, store_history},
  structs::{
    self,
    spotify::{ArtistName, CurrentPlaying, DeviceRewrite, PlayerState},
  },
  ServerState,
};

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

      if json.is_playing && json.item.is_some() {
        let item = json.item.unwrap();
        let mut image: String = String::from("none");
        let mut artists: Vec<ArtistName> = [].to_vec();

        if item.album.is_some() && item.artists.is_some() {
          image = item.album.unwrap().images[0].url.to_string();
          artists =
            item.artists.unwrap().into_iter().map(get_name).collect();
        } else if item.show.is_some() {
          let show = item.show.unwrap();
          image = show.images[0].url.to_string();
          artists = [ArtistName { name: show.name }].to_vec()
        }

        let current = CurrentPlaying {
          id: Some(item.id),
          name: Some(item.name),
          current_playing_type: Some(
            item.item_type.unwrap_or_else(|| String::from("track")),
          ),
          playing: json.is_playing,
          artists: Some(artists),
          length: Some(item.duration_ms),
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

          if current_playing.progress.unwrap() < 10000 {
            return;
          }

          match prisma
            .spotify_history()
            .find_first(vec![spotify_history::id::equals(
              current_playing.id.as_ref().unwrap().to_string(),
            )])
            .order_by(spotify_history::listened_at::order(Direction::Desc))
            .exec()
            .await
          {
            Ok(latest) => match latest {
              Some(latest) => {
                let listened_date = latest.listened_at.timestamp() * 1000;
                let date_minus_length =
                  (date.timestamp() * 1000) - latest.length as i64;

                if date_minus_length >= listened_date {
                  store_history(prisma, current_playing).await;
                }
              }
              None => store_history(prisma, current_playing).await,
            },
            Err(_) => store_history(prisma, current_playing).await,
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
