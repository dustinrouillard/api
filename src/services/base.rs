use actix_web::{get, http::Error, HttpResponse};

#[get("/")]
async fn index() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().body("Ok"))
}

#[get("/health")]
async fn health() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::NoContent().finish())
}
