use actix_web::{http::Error, post, HttpResponse};

#[post("/assets")]
async fn index() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().body("Ok"))
}
