use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstagramMe {
  pub media_count: i64,
  pub follows_count: i64,
  pub media: Media,
  pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
  pub data: Vec<Datum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Datum {
  pub id: String,
  pub caption: String,
  pub media_type: MediaType,
  pub media_product_type: MediaProductType,
  pub comments_count: i64,
  pub like_count: i64,
  pub media_url: String,
  pub thumbnail_url: Option<String>,
  pub permalink: String,
  pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaProductType {
  #[serde(rename = "FEED")]
  Feed,
  #[serde(rename = "REELS")]
  Reels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaType {
  #[serde(rename = "CAROUSEL_ALBUM")]
  CarouselAlbum,
  Image,
  Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstagramOverview {
  pub followers: i64,
  pub post_count: i64,
  pub posts: Vec<Datum>,
}

impl From<InstagramMe> for InstagramOverview {
  fn from(me: InstagramMe) -> Self {
    InstagramOverview {
      followers: me.follows_count,
      post_count: me.media_count,
      posts: me.media.data,
    }
  }
}
