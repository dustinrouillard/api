pub mod routes;

use actix_web::{web, Scope};

pub fn factory() -> Scope {
  web::scope("/photography")
    .service(routes::get_albums)
    .service(routes::get_album)
}
