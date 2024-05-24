use actix_multipart::form::bytes;

use once_cell::sync::Lazy;
use serde::Deserialize;

static ALLOWED_IMAGES: Lazy<Vec<&str>> = Lazy::new(|| {
  vec![
    "image/gif",
    "image/png",
    "image/svg",
    "image/webp",
    "image/jpeg",
  ]
});

static ALLOWED_FILES: Lazy<Vec<&str>> = Lazy::new(|| {
  let mut files = vec![
    "application/octet-stream",
    "application/ogg",
    "application/pdf",
    "application/rtf",
    "application/x-sh",
    "application/x-tar",
    "application/zip",
    "audio/mpeg",
    "audio/ogg",
    "audio/opus",
    "audio/wav",
    "audio/webm",
    "font/otf",
    "font/ttf",
    "font/woff",
    "font/woff2",
    "image/webp",
    "text/css",
    "text/csv",
    "text/sql",
    "text/javascript",
    "text/plain",
    "video/mov",
    "video/mp4",
    "video/mpeg",
    "video/ogg",
    "video/webm",
  ];
  files.extend(&*ALLOWED_IMAGES);
  files
});

#[derive(Deserialize, Debug)]
pub struct AssetType {
  pub mime: String,
  pub ext: String,
}

pub fn is_allowed_type(
  file: &bytes::Bytes,
  asset_type: String,
) -> (bool, AssetType) {
  let kind = infer::get(&file.data);

  match kind {
    Some(kind) => (
      (if asset_type == "images" {
        &ALLOWED_IMAGES
      } else {
        &ALLOWED_FILES
      })
      .contains(&kind.mime_type()),
      AssetType {
        mime: (&kind.mime_type()).to_string(),
        ext: (&kind.extension()).to_string(),
      },
    ),
    None => (
      false,
      AssetType {
        mime: "text/plain".to_string(),
        ext: "txt".to_string(),
      },
    ),
  }
}
