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
  pub frame: Field<Frame>,
}

/// Fields that can be applied across many photos at once. Each uses the
/// tri-state `Field` semantics: missing = leave unchanged, present-null =
/// clear, present-value = set.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BulkPhotoFields {
  #[serde(default)]
  pub caption: Field<String>,
  #[serde(default)]
  pub instagram: Field<String>,
  #[serde(default)]
  pub frame: Field<Frame>,
}

/// A single targeted photo update inside a bulk request.
#[derive(Debug, Serialize, Deserialize)]
pub struct BulkPhotoItem {
  pub name: String,
  #[serde(default)]
  pub caption: Field<String>,
  #[serde(default)]
  pub instagram: Field<String>,
  #[serde(default)]
  pub frame: Field<Frame>,
}

/// Bulk update payload. `apply_to_all` is applied first to every photo in the
/// album, then per-photo `photos` overrides are applied on top. Either or both
/// may be provided.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BulkUpdatePhotosPayload {
  #[serde(default)]
  pub apply_to_all: Option<BulkPhotoFields>,
  #[serde(default)]
  pub photos: Option<Vec<BulkPhotoItem>>,
}

/// Payload for reordering an album's photos. `order` must be a permutation of
/// the album's existing photo names (same set, no duplicates, no omissions).
#[derive(Debug, Serialize, Deserialize)]
pub struct ReorderPhotosPayload {
  pub order: Vec<String>,
}

/// Result of a single file within a multi-photo upload that was not stored.
#[derive(Debug, Serialize, Deserialize)]
pub struct SkippedUpload {
  pub name: Option<String>,
  pub reason: String,
}

/// Response for a multi-photo upload: the updated album plus a per-file
/// breakdown of what was stored and what was skipped.
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadPhotosResponse {
  pub album: PublicAlbum,
  pub uploaded: Vec<String>,
  pub skipped: Vec<SkippedUpload>,
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

/// Focal point for a photo, as `object-position` percentages (0–100).
/// `None`/absent means centered (50/50). Applies wherever the photo is
/// rendered, including the album cover slot when this photo is the cover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
  pub x: f64,
  pub y: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumItem {
  pub name: String,
  pub caption: Option<String>,
  pub instagram: Option<String>,
  // Optional: absent on older records, so it stays `Option` (serde defaults
  // missing `Option` fields to `None`). Serialized as `null` when centered.
  #[serde(default)]
  pub frame: Option<Frame>,
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
