use std::sync::Arc;

use actix_web::web;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{self, skip_serializing_none};
use sqlx::types::{chrono::DateTime, Json};

use crate::{
    connectivity::postgres::{SpotifyArtist, SpotifyHistoryItem},
    structs::{self, spotify::PlayerState},
    ServerState,
};

extern crate chrono;
extern crate serde_json;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CurrentPlaying {
    playing: bool,
    id: Option<String>,
    #[serde(rename = "type")]
    current_playing_type: Option<String>,
    name: Option<String>,
    artists: Option<Vec<Artist>>,
    length: Option<i64>,
    progress: Option<i64>,
    image: Option<String>,
    device: Option<Device>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Artist {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Device {
    name: String,
    #[serde(rename = "type")]
    device_type: String,
}

pub(crate) async fn fetch_spotify_current(data: web::Data<ServerState>) {
    let redis = &mut data.valkey.clone();
    let rabbit = &mut data.rabbit.clone();
    let postgres = &mut data.postgres.clone();

    match redis.get_spotify_account().await {
        Ok(account) => {
            let client = reqwest::Client::new();
            let res = client
                .get(format!(
                    "https://api.spotify.com/v1/me/player?additional_types=episode"
                ))
                .header("Authorization", format!("Bearer {}", account.access_token))
                .send()
                .await
                .unwrap();

            if res.status() != 200 {
                return redis.set_not_playing().await;
            }

            let json = res.json::<PlayerState>().await.unwrap();

            fn get_name(structs::spotify::Artist { name, .. }: structs::spotify::Artist) -> Artist {
                Artist { name }
            }

            if json.is_playing {
                let mut image: String = String::from("none");
                let mut artists: Vec<Artist> = [].to_vec();

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
                    artists = [Artist { name: show.name }].to_vec()
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
                    device: Some(Device {
                        name: json.device.name,
                        device_type: json
                            .device
                            .device_type
                            .unwrap_or_else(|| String::from("unknown")),
                    }),
                };

                let redis_current = redis.get_current().await;

                if redis_current != current {
                    let current_clone = Arc::new(current);

                    redis.update_current(&current_clone).await;
                    rabbit
                        .publish_spotify_current(&Arc::clone(&current_clone))
                        .await;

                    let date: DateTime<Utc> = Utc::now();
                    let current_playing = Arc::clone(&current_clone);
                    match postgres
                        .get_latest_history_item(current_playing.id.as_ref().unwrap().to_string())
                        .await
                    {
                        Ok(latest) => {
                            let listened_date = latest.listened_at.timestamp() * 1000;
                            let date_minus_length =
                                (date.timestamp() * 1000) - latest.length as i64;

                            if date_minus_length >= listened_date
                                && current_playing.progress.unwrap() >= 10000
                            {
                                let device = match postgres
                                    .get_device_by_name(
                                        current_playing.device.as_ref().unwrap().name.to_string(),
                                    )
                                    .await
                                {
                                    Ok(dev) => dev,
                                    Err(..) => postgres
                                        .insert_new_device(
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
                                        .await
                                        .unwrap(),
                                };

                                let mut artists = Vec::new();
                                for artist in current_playing.artists.as_ref().unwrap() {
                                    artists.push(Json(SpotifyArtist {
                                        name: artist.name.clone(),
                                    }));
                                }

                                let history_item = SpotifyHistoryItem {
                                    id: current_playing.id.as_ref().unwrap().to_string(),
                                    spotify_history_type: current_playing
                                        .current_playing_type
                                        .as_ref()
                                        .unwrap()
                                        .to_string(),
                                    name: current_playing.name.as_ref().unwrap().to_string(),
                                    length: *current_playing.length.as_ref().unwrap() as i32,
                                    image: current_playing.image.as_ref().unwrap().to_string(),
                                    listened_at: date,
                                    device: device.id,
                                    artists,
                                };

                                postgres.insert_new_history_item(history_item).await;
                            }
                        }
                        Err(..) => {
                            if current_playing.progress.unwrap() >= 10000 {
                                let device = match postgres
                                    .get_device_by_name(
                                        current_playing.device.as_ref().unwrap().name.to_string(),
                                    )
                                    .await
                                {
                                    Ok(dev) => dev,
                                    Err(..) => postgres
                                        .insert_new_device(
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
                                        .await
                                        .unwrap(),
                                };

                                let mut artists = Vec::new();
                                for artist in current_playing.artists.as_ref().unwrap() {
                                    artists.push(Json(SpotifyArtist {
                                        name: artist.name.clone(),
                                    }));
                                }

                                let history_item = SpotifyHistoryItem {
                                    id: current_playing.id.as_ref().unwrap().to_string(),
                                    spotify_history_type: current_playing
                                        .current_playing_type
                                        .as_ref()
                                        .unwrap()
                                        .to_string(),
                                    name: current_playing.name.as_ref().unwrap().to_string(),
                                    length: *current_playing.length.as_ref().unwrap() as i32,
                                    image: current_playing.image.as_ref().unwrap().to_string(),
                                    listened_at: date,
                                    device: device.id,
                                    artists,
                                };

                                postgres.insert_new_history_item(history_item).await;
                            }
                        }
                    }
                }
            } else {
                let redis_current = redis.get_current().await;

                if redis_current.playing {
                    redis.set_not_playing().await;
                    rabbit.publish_spotify_not_playing().await;
                }
            }
        }
        Err(..) => (),
    };
}
