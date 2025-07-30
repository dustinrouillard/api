use actix_multipart::form::MultipartForm;
use actix_web::{
  delete, error::ErrorInternalServerError, get, patch, post, put, web,
  Error, HttpResponse,
};
use optional_field::Field;
use prisma_client_rust::Direction;
use serde_json::json;

use crate::{
  connectivity::prisma::photography_albums::{self, OrderByParam},
  services::uploads::helpers,
  structs::{
    photography::{
      AlbumItem, CreateAlbumPayload, EditAlbumPayload, EditPhotoPayload,
      GetAlbumResponse, GetAlbumsResponse,
    },
    uploads::CdnUpload,
  },
  ServerState,
};

#[get("/albums")]
async fn get_albums(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let albums = state
    .prisma
    .photography_albums()
    .find_many(vec![])
    .order_by(OrderByParam::Date(Direction::Asc))
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to fetch albums from database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  let albums = albums
    .into_iter()
    .filter(|album| {
      serde_json::from_value::<Vec<AlbumItem>>(album.items.clone())
        .unwrap()
        .len()
        > 0
    })
    .collect::<Vec<_>>();

  Ok(HttpResponse::Ok().json(GetAlbumsResponse::from(albums)))
}

#[get("/albums/{slug}")]
async fn get_album(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let album = state
    .prisma
    .photography_albums()
    .find_first(vec![photography_albums::slug::equals(slug.into_inner())])
    .order_by(OrderByParam::Date(Direction::Asc))
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to fetch albums from database {:?}", error);
      return ErrorInternalServerError(error.to_string());
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
  let album = state
    .prisma
    .photography_albums()
    .create(payload.slug.clone(), payload.name.clone(), vec![])
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to create album in database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  Ok(HttpResponse::Created().json(GetAlbumResponse::from(album)))
}

#[patch("/albums/{slug}")]
async fn edit_album(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
  payload: web::Json<EditAlbumPayload>,
) -> Result<HttpResponse, Error> {
  let album = state
    .prisma
    .photography_albums()
    .find_unique(photography_albums::slug::equals(slug.clone()))
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to get album from database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  if album.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
    );
  }

  let mut params: Vec<Option<photography_albums::SetParam>> =
    vec![payload.name.clone().map(photography_albums::name::set)];

  match &payload.description {
    Field::Missing => {}
    Field::Present(None) => {
      params.push(Some(photography_albums::description::set(None)))
    }
    Field::Present(description) => {
      params.push(Some(photography_albums::description::set(
        description.to_owned(),
      )));
    }
  }

  match &payload.location {
    Field::Missing => {}
    Field::Present(None) => {
      params.push(Some(photography_albums::location::set(None)))
    }
    Field::Present(location) => {
      params.push(Some(photography_albums::location::set(
        location.to_owned(),
      )));
    }
  }

  let items: Vec<AlbumItem> = serde_json::from_value(album.unwrap().items)
    .map_err(|error| {
      eprintln!("failed to get album items {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  match &payload.cover {
    Field::Missing => {}
    Field::Present(None) => {
      params.push(Some(photography_albums::cover::set(None)))
    }
    Field::Present(cover) => {
      let name = cover.to_owned().unwrap();
      let photo = items.iter().find(|item| item.name == name);
      if photo.is_none() {
        return Ok(
          HttpResponse::NotFound()
            .json(json!({"code": "cover_photo_not_found"})),
        );
      }

      params.push(Some(photography_albums::cover::set(Some(name))));
    }
  }

  let params: Vec<photography_albums::SetParam> =
    params.into_iter().flatten().collect();

  let album = state
    .prisma
    .photography_albums()
    .update(photography_albums::slug::equals(slug.into_inner()), params)
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to create album in database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  Ok(HttpResponse::Created().json(GetAlbumResponse::from(album)))
}

#[delete("/albums/{slug}")]
async fn delete_album(
  state: web::Data<ServerState>,
  slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let album = state
    .prisma
    .photography_albums()
    .find_unique(photography_albums::slug::equals(slug.clone()))
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to get album from database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  if album.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
    );
  }

  let album = album.unwrap();
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

  state
    .prisma
    .photography_albums()
    .delete(photography_albums::slug::equals(slug.into_inner()))
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to delete album in database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  Ok(HttpResponse::NoContent().finish())
}

#[put("/albums/{slug}")]
async fn upload_photo(
  MultipartForm(form): MultipartForm<CdnUpload>,
  state: web::Data<ServerState>,
  slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let album = state
    .prisma
    .photography_albums()
    .find_unique(photography_albums::slug::equals(slug.clone()))
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to get album from database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  if album.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
    );
  }

  let file = &mut form.files.first().unwrap();
  let file_type =
    match helpers::is_allowed_type(file, "images".to_string()) {
      (true, res) => res,
      (false, _) => {
        return Ok(
          HttpResponse::BadRequest()
            .json(json!({"code": "prohibited_file_type"})),
        );
      }
    };

  if file.file_name.is_none() {
    return Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "missing_file_name"})),
    );
  }

  let file_name = file.file_name.clone().unwrap();

  let mut items: Vec<AlbumItem> =
    serde_json::from_value(album.unwrap().items).map_err(|error| {
      eprintln!("failed to parse album items {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  let photo = items.iter_mut().find(|item| item.name == file_name);
  if photo.is_some() {
    return Ok(
      HttpResponse::NotFound()
        .json(json!({"code": "photo_already_exists"})),
    );
  }

  let path = format!("gallery/albums/{}/{}", slug.clone(), file_name);

  let s3 = &state.s3;
  let response = s3
    .cdn_bucket
    .put_object_with_content_type(&path, &file.data, &file_type.mime)
    .await
    .unwrap();

  if response.status_code() != 200 {
    return Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "failed_upload_to_s3"})),
    );
  }

  items.push(AlbumItem {
    name: file_name,
    caption: None,
    instagram: None,
  });

  state
    .prisma
    .photography_albums()
    .update(
      photography_albums::slug::equals(slug.into_inner()),
      vec![photography_albums::items::set(
        serde_json::to_value(items).unwrap(),
      )],
    )
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to update album in database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  Ok(HttpResponse::NoContent().finish())
}

#[patch("/albums/{slug}/photos/{name}")]
async fn update_photo(
  state: web::Data<ServerState>,
  path: web::Path<(String, String)>,
  payload: web::Json<EditPhotoPayload>,
) -> Result<HttpResponse, Error> {
  let (slug, name) = path.into_inner();

  let album = state
    .prisma
    .photography_albums()
    .find_unique(photography_albums::slug::equals(slug.clone()))
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to get album from database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  if album.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
    );
  }

  let mut items: Vec<AlbumItem> =
    serde_json::from_value(album.unwrap().items).map_err(|error| {
      eprintln!("failed to get album items {:?}", error);
      return ErrorInternalServerError(error.to_string());
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

  let album = state
    .prisma
    .photography_albums()
    .update(
      photography_albums::slug::equals(slug),
      vec![photography_albums::items::set(
        serde_json::to_value(items).unwrap(),
      )],
    )
    .exec()
    .await
    .map_err(|error| {
      eprintln!("failed to update album in database {:?}", error);
      return ErrorInternalServerError(error.to_string());
    })?;

  Ok(HttpResponse::Ok().json(GetAlbumResponse::from(album)))
}

#[delete("/albums/{slug}/photos/{name}")]
async fn delete_photo(
  state: web::Data<ServerState>,
  path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
  let (slug, name) = path.into_inner();

  let album = state
    .prisma
    .photography_albums()
    .find_unique(photography_albums::slug::equals(slug.clone()))
    .exec()
    .await
    .map_err(|error| ErrorInternalServerError(error.to_string()))?;

  if album.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "album_not_found"})),
    );
  }

  let album = album.unwrap();

  let mut items: Vec<AlbumItem> = serde_json::from_value(album.items)
    .map_err(|error| ErrorInternalServerError(error.to_string()))?;

  let photo_index = items.iter().position(|item| item.name == name);
  if photo_index.is_none() {
    return Ok(
      HttpResponse::NotFound().json(json!({"code": "photo_not_found"})),
    );
  }

  let photo = items.remove(photo_index.unwrap());

  let mut params: Vec<photography_albums::SetParam> =
    vec![photography_albums::items::set(
      serde_json::to_value(&items).unwrap(),
    )];

  if Some(photo.name.clone()) == album.cover {
    params.push(photography_albums::cover::set(None));
  }

  let path = format!("gallery/albums/{}/{}", slug.clone(), name);
  let s3 = &state.s3;
  let response = s3.cdn_bucket.delete_object(&path).await.unwrap();
  if response.status_code() != 204 {
    return Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "failed_delete_from_s3"})),
    );
  }

  state
    .prisma
    .photography_albums()
    .update(photography_albums::slug::equals(slug), params)
    .exec()
    .await
    .map_err(|error| ErrorInternalServerError(error.to_string()))?;

  Ok(HttpResponse::NoContent().finish())
}
