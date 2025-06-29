use crate::services;
use actix_web::{web, Scope};

pub fn factory() -> Scope {
  web::scope("/weather")
    .service(services::weather::routes::get_current_weather)
}
