use optional_field::{serde_optional_fields, Field};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::connectivity::prisma::photography_albums::Data;

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAlbumsResponse {
  pub albums: Vec<PublicAlbum>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAlbumResponse {
  pub album: PublicAlbum,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAlbumPayload {
  pub name: String,
  pub slug: String,
  pub cover: Option<String>,
  pub location: Option<String>,
  pub description: Option<String>,
}

#[serde_optional_fields]
#[derive(Debug, Serialize, Deserialize)]
pub struct EditAlbumPayload {
  pub name: Option<String>,
  pub cover: Field<String>,
  pub location: Field<String>,
  pub description: Field<String>,
}

#[serde_optional_fields]
#[derive(Debug, Serialize, Deserialize)]
pub struct EditPhotoPayload {
  pub caption: Field<String>,
  pub instagram: Field<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicAlbum {
  pub slug: String,
  pub name: String,
  pub cover: Option<String>,
  pub description: Option<String>,
  pub location: Option<String>,
  pub items: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumItem {
  pub name: String,
  pub caption: Option<String>,
  pub instagram: Option<String>,
}

impl From<Data> for PublicAlbum {
  fn from(albums: Data) -> Self {
    Self {
      slug: albums.slug,
      name: albums.name,
      cover: albums.cover,
      description: albums.description,
      location: albums.location,
      items: albums.items,
    }
  }
}

impl From<Vec<Data>> for GetAlbumsResponse {
  fn from(albums: Vec<Data>) -> Self {
    Self {
      albums: albums
        .into_iter()
        .map(|album| PublicAlbum::from(album))
        .collect(),
    }
  }
}

impl From<Data> for GetAlbumResponse {
  fn from(album: Data) -> Self {
    Self {
      album: PublicAlbum::from(album),
    }
  }
}
