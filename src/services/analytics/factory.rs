use actix_web::{web, Scope};
use actix_web_lab::middleware::from_fn;

use crate::services;

pub fn factory() -> Scope {
  web::scope("/analytics")
    .service(services::analytics::routes::get_analytics)
    .service(
      web::scope("")
        .wrap(from_fn(services::middleware::auth_middleware))
        .service(services::analytics::routes::track_command),
    )
}
