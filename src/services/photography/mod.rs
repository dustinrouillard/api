pub mod routes;

use actix_web::{web, Scope};
use actix_web_lab::middleware::from_fn;

use crate::services;

pub fn factory() -> Scope {
  web::scope("/photography")
    .service(routes::get_albums)
    .service(routes::get_album)
    .service(
      web::scope("")
        .wrap(from_fn(services::middleware::auth_middleware))
        .service(routes::create_album)
        .service(routes::edit_album)
        .service(routes::delete_album)
        .service(routes::upload_photo)
        .service(routes::delete_photo)
        .service(routes::update_photo),
    )
}
