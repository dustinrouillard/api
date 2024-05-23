use actix_web::{delete, get, http::Error, post, HttpResponse};

#[post("/posts/{id}/assets")]
async fn upload_asset_for_post() -> Result<HttpResponse, Error> {
  Ok(HttpResponse::NotImplemented().finish())
}

#[get("/posts/{id}/assets")]
async fn get_assets_for_post() -> Result<HttpResponse, Error> {
  Ok(HttpResponse::NotImplemented().finish())
}

#[delete("/posts/{id}/assets/{hash}")]
async fn delete_asset_for_post() -> Result<HttpResponse, Error> {
  Ok(HttpResponse::NotImplemented().finish())
}
