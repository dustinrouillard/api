use actix_web::{web, Scope};
use actix_web_lab::middleware::from_fn;

use crate::services;

pub fn analytics_factory() -> Scope {
  web::scope("/analytics")
    .service(services::analytics::routes::get_analytics)
    .service(
      web::scope("")
        .wrap(from_fn(services::uploads::middleware::uploads_auth_mw))
        .service(services::analytics::routes::track_command),
    )
}
