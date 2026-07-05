use actix_multipart::form::MultipartForm;
use actix_web::{
  delete, error::ErrorInternalServerError, get,
  http::header::AUTHORIZATION, patch, post, put, web, Error, HttpRequest,
  HttpResponse,
};
use optional_field::Field;
use serde_json::json;

use crate::{
  helpers::authentication::is_management_authed,
  services::uploads::helpers,
  structs::{
    photography::{
      Album, AlbumItem, BulkUpdatePhotosPayload, CreateAlbumPayload,
      EditAlbumPayload, EditPhotoPayload, GetAlbumResponse,
      GetAlbumsResponse, PublicAlbum, ReorderPhotosPayload, SkippedUpload,
      UploadPhotosResponse,
    },
    uploads::CdnUpload,
  },
  ServerState,
};
use std::collections::HashMap;

/// Fetch a single album by its slug.
async fn find_album(
  db: &sqlx::PgPool,
  slug: &str,
) -> Result<Option<Album>, sqlx::Error> {
  sqlx::query_as::<_, Album>(
    "SELECT * FROM photography_albums WHERE slug = $1 LIMIT 1",
  )
  .bind(slug)
  .fetch_optional(db)
  .await
}

#[get("/albums")]
async fn get_albums(
  state: web::Data<ServerState>,
  req: HttpRequest,
) -> Result<HttpResponse, Error> {
  let albums = sqlx::query_as::<_, Album>(
    "SELECT * FROM photography_albums ORDER BY date ASC",
  )
  .fetch_all(&state.db)
  .await
  .map_err(|error| {
    eprintln!("failed to fetch albums from database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })?;

  let valkey = &mut state.valkey.clone();
  let is_management_authed =
    is_management_authed(valkey, req.headers().get(AUTHORIZATION))
      .await
      .ok();

  let albums = albums
    .into_iter()
    .filter(|album| {
      if let Some(true) = is_management_authed {
        true
      } else {
        serde_json::from_value::<Vec<AlbumItem>>(album.items.clone())
          .unwrap()
          .len()
          > 0
      }
    })
    .collect::<Vec<_>>();

  Ok(HttpResponse::Ok().json(GetAlbumsResponse::from(albums)))
}

#[get("/albums/{slug}")]
async fn get_album(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let album = find_album(&state.db, &slug.into_inner())
    .await
    .map_err(|error| {
      eprintln!("failed to fetch albums from database {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?;

  if album.is_none() {
    return Ok(HttpResponse::NotFound().finish());
  }

  Ok(HttpResponse::Ok().json(GetAlbumResponse::from(album.unwrap())))
}

#[post("/albums")]
async fn create_album(
  state: web::Data<ServerState>,
  payload: web::Json<CreateAlbumPayload>,
) -> Result<HttpResponse, Error> {
  let album = sqlx::query_as::<_, Album>(
    "INSERT INTO photography_albums (slug, name, location, description) \
     VALUES ($1, $2, $3, $4) RETURNING *",
  )
  .bind(payload.slug.clone())
  .bind(payload.name.clone())
  .bind(payload.location.clone())
  .bind(payload.description.clone())
  .fetch_one(&state.db)
  .await
  .map_err(|error| {
    eprintln!("failed to create album in database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })?;

  Ok(HttpResponse::Created().json(GetAlbumResponse::from(album)))
}

#[patch("/albums/{slug}")]
async fn edit_album(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
  payload: web::Json<EditAlbumPayload>,
) -> Result<HttpResponse, Error> {
  let slug = slug.into_inner();

  let album = match find_album(&state.db, &slug).await.map_err(|error| {
    eprintln!("failed to get album from database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })? {
    Some(album) => album,
    None => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
      );
    }
  };

  let items: Vec<AlbumItem> = serde_json::from_value(album.items.clone())
    .map_err(|error| {
      eprintln!("failed to get album items {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?;

  // Resolve each column to its final value (tri-state fields can clear to
  // NULL; missing fields keep the current value).
  let name = payload.name.clone().unwrap_or(album.name);
  let description = match &payload.description {
    Field::Missing => album.description,
    Field::Present(None) => None,
    Field::Present(description) => description.to_owned(),
  };
  let location = match &payload.location {
    Field::Missing => album.location,
    Field::Present(None) => None,
    Field::Present(location) => location.to_owned(),
  };
  let cover = match &payload.cover {
    Field::Missing => album.cover,
    Field::Present(None) => None,
    Field::Present(cover) => {
      let cover_name = cover.to_owned().unwrap();
      if !items.iter().any(|item| item.name == cover_name) {
        return Ok(
          HttpResponse::NotFound()
            .json(json!({"code": "cover_photo_not_found"})),
        );
      }
      Some(cover_name)
    }
  };

  let album = sqlx::query_as::<_, Album>(
    "UPDATE photography_albums \
     SET name = $1, description = $2, location = $3, cover = $4 \
     WHERE slug = $5 RETURNING *",
  )
  .bind(name)
  .bind(description)
  .bind(location)
  .bind(cover)
  .bind(&slug)
  .fetch_one(&state.db)
  .await
  .map_err(|error| {
    eprintln!("failed to update album in database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })?;

  Ok(HttpResponse::Ok().json(GetAlbumResponse::from(album)))
}

#[delete("/albums/{slug}")]
async fn delete_album(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let slug = slug.into_inner();

  let album = match find_album(&state.db, &slug).await.map_err(|error| {
    eprintln!("failed to get album from database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })? {
    Some(album) => album,
    None => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
      );
    }
  };

  let items: Vec<AlbumItem> = serde_json::from_value(album.items)
    .map_err(|error| ErrorInternalServerError(error.to_string()))?;

  if items.len() > 0 {
    let path = format!("gallery/albums/{}", slug.clone());
    let s3 = &state.s3;

    let list_res = s3
      .cdn_bucket
      .list(path.to_string(), None)
      .await
      .map_err(|error| ErrorInternalServerError(error.to_string()))?;

    let files: Vec<String> = list_res
      .into_iter()
      .flat_map(|result| result.contents)
      .map(|object| object.key)
      .collect();

    for file in files {
      let response = s3.cdn_bucket.delete_object(&file).await.unwrap();
      if response.status_code() != 204 {
        return Ok(
          HttpResponse::BadRequest()
            .json(json!({"code": "failed_delete_from_s3"})),
        );
      }
    }
  }

  sqlx::query("DELETE FROM photography_albums WHERE slug = $1")
    .bind(&slug)
    .execute(&state.db)
    .await
    .map_err(|error| {
      eprintln!("failed to delete album in database {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?;

  Ok(HttpResponse::NoContent().finish())
}

#[put("/albums/{slug}")]
async fn upload_photos(
  MultipartForm(form): MultipartForm<CdnUpload>,
  state: web::Data<ServerState>,
  slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let slug = slug.into_inner();

  let album = match find_album(&state.db, &slug).await.map_err(|error| {
    eprintln!("failed to get album from database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })? {
    Some(album) => album,
    None => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
      );
    }
  };

  if form.files.is_empty() {
    return Ok(
      HttpResponse::BadRequest().json(json!({"code": "no_files"})),
    );
  }

  let mut items: Vec<AlbumItem> =
    serde_json::from_value(album.items.clone()).map_err(|error| {
      eprintln!("failed to parse album items {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?;

  let s3 = &state.s3;
  let mut uploaded: Vec<String> = Vec::new();
  let mut skipped: Vec<SkippedUpload> = Vec::new();

  for file in form.files.iter() {
    let file_type =
      match helpers::is_allowed_type(file, "images".to_string()) {
        (true, res) => res,
        (false, _) => {
          skipped.push(SkippedUpload {
            name: file.file_name.clone(),
            reason: "prohibited_file_type".to_string(),
          });
          continue;
        }
      };

    let file_name = match &file.file_name {
      Some(name) => name.clone(),
      None => {
        skipped.push(SkippedUpload {
          name: None,
          reason: "missing_file_name".to_string(),
        });
        continue;
      }
    };

    // Guard against clashing with a photo that already exists, or with
    // another file of the same name earlier in this same request.
    if items.iter().any(|item| item.name == file_name) {
      skipped.push(SkippedUpload {
        name: Some(file_name),
        reason: "photo_already_exists".to_string(),
      });
      continue;
    }

    let path = format!("gallery/albums/{}/{}", slug.clone(), file_name);
    let response = s3
      .cdn_bucket
      .put_object_with_content_type(&path, &file.data, &file_type.mime)
      .await;

    match response {
      Ok(response) if response.status_code() == 200 => {
        items.push(AlbumItem {
          name: file_name.clone(),
          caption: None,
          instagram: None,
          frame: None,
        });
        uploaded.push(file_name);
      }
      _ => {
        skipped.push(SkippedUpload {
          name: Some(file_name),
          reason: "failed_upload_to_s3".to_string(),
        });
      }
    }
  }

  // Only touch the database if at least one file actually landed.
  let album = if uploaded.is_empty() {
    album
  } else {
    sqlx::query_as::<_, Album>(
      "UPDATE photography_albums SET items = $1 WHERE slug = $2 \
       RETURNING *",
    )
    .bind(serde_json::to_value(&items).unwrap())
    .bind(&slug)
    .fetch_one(&state.db)
    .await
    .map_err(|error| {
      eprintln!("failed to update album in database {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?
  };

  Ok(HttpResponse::Ok().json(UploadPhotosResponse {
    album: PublicAlbum::from(album),
    uploaded,
    skipped,
  }))
}

#[patch("/albums/{slug}/photos/{name}")]
async fn update_photo(
  state: web::Data<ServerState>,
  path: web::Path<(String, String)>,
  payload: web::Json<EditPhotoPayload>,
) -> Result<HttpResponse, Error> {
  let (slug, name) = path.into_inner();

  let album = match find_album(&state.db, &slug).await.map_err(|error| {
    eprintln!("failed to get album from database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })? {
    Some(album) => album,
    None => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
      );
    }
  };

  let mut items: Vec<AlbumItem> = serde_json::from_value(album.items)
    .map_err(|error| {
      eprintln!("failed to get album items {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?;

  let photo = items.iter_mut().find(|item| item.name == name);
  if photo.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "photo_not_found"})),
    );
  }

  let photo = photo.unwrap();
  match &payload.caption {
    Field::Missing => (),
    Field::Present(None) => photo.caption = None,
    Field::Present(caption) => photo.caption = caption.clone(),
  }
  match &payload.instagram {
    Field::Missing => (),
    Field::Present(None) => photo.instagram = None,
    Field::Present(instagram) => photo.instagram = instagram.clone(),
  }
  match &payload.frame {
    Field::Missing => (),
    Field::Present(None) => photo.frame = None,
    Field::Present(frame) => photo.frame = frame.clone(),
  }

  let album = sqlx::query_as::<_, Album>(
    "UPDATE photography_albums SET items = $1 WHERE slug = $2 RETURNING *",
  )
  .bind(serde_json::to_value(items).unwrap())
  .bind(&slug)
  .fetch_one(&state.db)
  .await
  .map_err(|error| {
    eprintln!("failed to update album in database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })?;

  Ok(HttpResponse::Ok().json(GetAlbumResponse::from(album)))
}

#[patch("/albums/{slug}/photos")]
async fn bulk_update_photos(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
  payload: web::Json<BulkUpdatePhotosPayload>,
) -> Result<HttpResponse, Error> {
  let slug = slug.into_inner();

  let album = match find_album(&state.db, &slug).await.map_err(|error| {
    eprintln!("failed to get album from database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })? {
    Some(album) => album,
    None => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
      );
    }
  };

  let mut items: Vec<AlbumItem> = serde_json::from_value(album.items)
    .map_err(|error| {
      eprintln!("failed to get album items {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?;

  // Validate every targeted photo exists before mutating anything, so the
  // request is all-or-nothing.
  if let Some(photos) = &payload.photos {
    let missing: Vec<String> = photos
      .iter()
      .filter(|update| !items.iter().any(|item| item.name == update.name))
      .map(|update| update.name.clone())
      .collect();

    if !missing.is_empty() {
      return Ok(HttpResponse::NotFound().json(
        json!({"code": "photos_not_found", "photos": missing}),
      ));
    }
  }

  // First, blanket changes across every photo in the album.
  if let Some(all) = &payload.apply_to_all {
    for item in items.iter_mut() {
      match &all.caption {
        Field::Missing => (),
        Field::Present(None) => item.caption = None,
        Field::Present(caption) => item.caption = caption.clone(),
      }
      match &all.instagram {
        Field::Missing => (),
        Field::Present(None) => item.instagram = None,
        Field::Present(instagram) => item.instagram = instagram.clone(),
      }
      match &all.frame {
        Field::Missing => (),
        Field::Present(None) => item.frame = None,
        Field::Present(frame) => item.frame = frame.clone(),
      }
    }
  }

  // Then per-photo overrides on top.
  if let Some(photos) = &payload.photos {
    for update in photos {
      if let Some(item) =
        items.iter_mut().find(|item| item.name == update.name)
      {
        match &update.caption {
          Field::Missing => (),
          Field::Present(None) => item.caption = None,
          Field::Present(caption) => item.caption = caption.clone(),
        }
        match &update.instagram {
          Field::Missing => (),
          Field::Present(None) => item.instagram = None,
          Field::Present(instagram) => item.instagram = instagram.clone(),
        }
        match &update.frame {
          Field::Missing => (),
          Field::Present(None) => item.frame = None,
          Field::Present(frame) => item.frame = frame.clone(),
        }
      }
    }
  }

  let album = sqlx::query_as::<_, Album>(
    "UPDATE photography_albums SET items = $1 WHERE slug = $2 RETURNING *",
  )
  .bind(serde_json::to_value(items).unwrap())
  .bind(&slug)
  .fetch_one(&state.db)
  .await
  .map_err(|error| {
    eprintln!("failed to update album in database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })?;

  Ok(HttpResponse::Ok().json(GetAlbumResponse::from(album)))
}

#[put("/albums/{slug}/order")]
async fn reorder_photos(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
  payload: web::Json<ReorderPhotosPayload>,
) -> Result<HttpResponse, Error> {
  let slug = slug.into_inner();

  let album = match find_album(&state.db, &slug).await.map_err(|error| {
    eprintln!("failed to get album from database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })? {
    Some(album) => album,
    None => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
      );
    }
  };

  let items: Vec<AlbumItem> = serde_json::from_value(album.items)
    .map_err(|error| {
      eprintln!("failed to get album items {:?}", error);
      ErrorInternalServerError(error.to_string())
    })?;

  // The new order must be an exact permutation of the existing photos:
  // no duplicates, and the same set of names (which also enforces equal
  // length). Build a lookup we can drain from so each name is consumed once.
  let mut lookup: HashMap<String, AlbumItem> =
    items.into_iter().map(|item| (item.name.clone(), item)).collect();

  let mut reordered: Vec<AlbumItem> = Vec::with_capacity(lookup.len());
  for name in &payload.order {
    match lookup.remove(name) {
      Some(item) => reordered.push(item),
      None => {
        // Either an unknown photo, or a duplicate that was already drained.
        return Ok(HttpResponse::BadRequest().json(
          json!({"code": "invalid_order", "photo": name}),
        ));
      }
    }
  }

  // Anything left in the lookup means the caller omitted a photo.
  if !lookup.is_empty() {
    let missing: Vec<String> = lookup.into_keys().collect();
    return Ok(HttpResponse::BadRequest().json(
      json!({"code": "incomplete_order", "missing": missing}),
    ));
  }

  let album = sqlx::query_as::<_, Album>(
    "UPDATE photography_albums SET items = $1 WHERE slug = $2 RETURNING *",
  )
  .bind(serde_json::to_value(reordered).unwrap())
  .bind(&slug)
  .fetch_one(&state.db)
  .await
  .map_err(|error| {
    eprintln!("failed to update album in database {:?}", error);
    ErrorInternalServerError(error.to_string())
  })?;

  Ok(HttpResponse::Ok().json(GetAlbumResponse::from(album)))
}

#[delete("/albums/{slug}/photos/{name}")]
async fn delete_photo(
  state: web::Data<ServerState>,
  path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
  let (slug, name) = path.into_inner();

  let album = match find_album(&state.db, &slug)
    .await
    .map_err(|error| ErrorInternalServerError(error.to_string()))?
  {
    Some(album) => album,
    None => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
      );
    }
  };

  let mut items: Vec<AlbumItem> =
    serde_json::from_value(album.items.clone())
      .map_err(|error| ErrorInternalServerError(error.to_string()))?;

  let photo_index = items.iter().position(|item| item.name == name);
  if photo_index.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "photo_not_found"})),
    );
  }

  let photo = items.remove(photo_index.unwrap());

  // Clear the cover if we just deleted the photo it referenced.
  let cover = if album.cover.as_deref() == Some(photo.name.as_str()) {
    None
  } else {
    album.cover
  };

  let path = format!("gallery/albums/{}/{}", slug.clone(), name);
  let s3 = &state.s3;
  let response = s3.cdn_bucket.delete_object(&path).await.unwrap();
  if response.status_code() != 204 {
    return Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "failed_delete_from_s3"})),
    );
  }

  sqlx::query(
    "UPDATE photography_albums SET items = $1, cover = $2 WHERE slug = $3",
  )
  .bind(serde_json::to_value(&items).unwrap())
  .bind(cover)
  .bind(&slug)
  .execute(&state.db)
  .await
  .map_err(|error| ErrorInternalServerError(error.to_string()))?;

  Ok(HttpResponse::NoContent().finish())
}
