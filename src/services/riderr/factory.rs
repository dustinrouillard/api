use actix_web::{web, Scope};

use crate::services;

pub fn riderr_factory() -> Scope {
  web::scope("/riderr").service(services::riderr::routes::ride_stats)
}
