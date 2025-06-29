use actix_web::{web, Scope};

use crate::services;

pub fn factory() -> Scope {
  web::scope("/spotify")
    .service(services::spotify::routes::recent_listens)
    .service(services::spotify::routes::current)
    .service(services::spotify::routes::authorize)
    .service(services::spotify::routes::setup)
}
