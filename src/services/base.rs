use actix_web::{get, http::Error, HttpResponse};
use serde_json::json;

pub async fn index() -> Result<HttpResponse, Error> {
  Ok(
    HttpResponse::NotFound()
      .append_header(("Content-type", "application/json"))
      .body(json!({"code": "route_not_found"}).to_string()),
  )
}

#[get("/health")]
async fn health() -> Result<HttpResponse, Error> {
  Ok(HttpResponse::NoContent().finish())
}
