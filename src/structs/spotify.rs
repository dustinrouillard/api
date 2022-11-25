use serde::{Deserialize, Serialize};
extern crate serde_json;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub device: Device,
    pub shuffle_state: bool,
    pub repeat_state: Option<String>,
    pub timestamp: i64,
    pub context: Option<Context>,
    pub progress_ms: i64,
    pub item: Item,
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
