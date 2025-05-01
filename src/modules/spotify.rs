use std::sync::Arc;

use actix_web::web::{self};
use chrono::prelude::*;
use prisma_client_rust::Direction;

use crate::{
  connectivity::prisma::spotify_history,
  services::spotify::helpers::{self, get_player_state, store_history},
  structs::{
    self,
    spotify::{ArtistName, CurrentPlaying, DeviceRewrite},
  },
  ServerState,
};

fn get_name(
  structs::spotify::Artist { name, .. }: structs::spotify::Artist,
) -> ArtistName {
  ArtistName { name }
}

pub(crate) async fn fetch_spotify_current(data: web::Data<ServerState>) {
  let valkey = &mut data.valkey.clone();
  let rabbit = &mut data.rabbit.clone();

  let prisma = &mut &data.prisma;

  let mut alt = Some(false);
  let player_state =
    if let Ok(main_player) = get_player_state(valkey, None).await {
      if main_player.is_playing {
        Some(main_player)
      } else {
        if let Ok(player) = get_player_state(valkey, Some(true)).await {
          if player.is_playing {
            alt = Some(true);
            Some(player)
          } else {
            Some(main_player)
          }
        } else {
          Some(main_player)
        }
      }
    } else {
      None
    };

  if let Some(player) = player_state {
    if player.is_playing && player.item.is_some() {
      let item = player.item.unwrap();
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
        playing: player.is_playing,
        artists: Some(artists),
        length: Some(item.duration_ms),
        progress: Some(player.progress_ms),
        image: Some(image),
        device: Some(DeviceRewrite {
          name: player.device.name,
          device_type: player
            .device
            .device_type
            .unwrap_or_else(|| String::from("unknown")),
        }),
        alt,
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
  } else {
    helpers::set_not_playing(valkey).await;
  }
}
