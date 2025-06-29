use actix_web::{
  error::ErrorInternalServerError, get, web, Error, HttpResponse,
};
use prisma_client_rust::Direction;

use crate::{
  connectivity::prisma::photography_albums::{self, OrderByParam},
  structs::photography::{GetAlbumResponse, GetAlbumsResponse},
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
