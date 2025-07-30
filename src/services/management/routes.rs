use actix_web::{get, http::Error, HttpResponse};

#[get("/check")]
async fn check_token() -> Result<HttpResponse, Error> {
  Ok(HttpResponse::NoContent().finish())
}
