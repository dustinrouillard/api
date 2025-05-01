use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
extern crate serde_json;

#[derive(Deserialize, Debug)]
pub struct RecentSongQuery {
  pub limit: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct SpotifyQueryString {
  pub alt: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerState {
  pub device: Device,
  pub shuffle_state: bool,
  pub repeat_state: Option<String>,
  pub timestamp: i64,
  pub context: Option<Context>,
  pub progress_ms: i64,
  pub item: Option<Item>,
  #[serde(rename = "type")]
  pub currently_playing_type: Option<String>,
  pub is_playing: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
  pub external_urls: ExternalUrls,
  pub href: String,
  #[serde(rename = "type")]
  pub context_type: Option<String>,
  pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalUrls {
  pub spotify: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
  pub id: String,
  pub is_active: bool,
  pub is_private_session: bool,
  pub is_restricted: bool,
  pub name: String,
  #[serde(rename = "type")]
  pub device_type: Option<String>,
  pub volume_percent: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
  pub album: Option<Album>,
  pub show: Option<Show>,
  pub artists: Option<Vec<Artist>>,
  pub duration_ms: i64,
  pub explicit: bool,
  pub external_urls: ExternalUrls,
  pub href: String,
  pub id: String,
  pub name: String,
  pub popularity: Option<i64>,
  pub track_number: Option<i64>,
  #[serde(rename = "type")]
  pub item_type: Option<String>,
  pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Album {
  #[serde(rename = "type")]
  pub album_type: Option<String>,
  pub artists: Option<Vec<Artist>>,
  pub external_urls: ExternalUrls,
  pub href: String,
  pub id: String,
  pub images: Vec<Image>,
  pub name: String,
  pub release_date: String,
  pub release_date_precision: String,
  pub total_tracks: i64,
  pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Show {
  #[serde(rename = "type")]
  pub show_type: Option<String>,
  pub external_urls: ExternalUrls,
  pub href: String,
  pub id: String,
  pub images: Vec<Image>,
  pub name: String,
  pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artist {
  pub external_urls: ExternalUrls,
  pub href: String,
  pub id: String,
  pub name: String,
  #[serde(rename = "type")]
  pub artist_type: Option<String>,
  pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
  pub height: i64,
  pub url: String,
  pub width: i64,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CurrentPlaying {
  pub playing: bool,
  pub id: Option<String>,
  #[serde(rename = "type")]
  pub current_playing_type: Option<String>,
  pub name: Option<String>,
  pub artists: Option<Vec<ArtistName>>,
  pub length: Option<i64>,
  pub progress: Option<i64>,
  pub image: Option<String>,
  pub device: Option<DeviceRewrite>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ArtistName {
  pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DeviceRewrite {
  pub name: String,
  #[serde(rename = "type")]
  pub device_type: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SpotifyAccount {
  pub access_token: String,
  pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AuthorizationData {
  pub code: Option<String>,
  pub refresh_token: Option<String>,
  pub grant_type: String,
  pub redirect_uri: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize)]
pub struct SpotifyArtist {
  pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyTokens {
  pub access_token: String,
  pub token_type: String,
  pub expires_in: u32,
  pub refresh_token: Option<String>,
  pub scope: String,
}
