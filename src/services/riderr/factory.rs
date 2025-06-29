use actix_web::{web, Scope};

use crate::services;

pub fn factory() -> Scope {
  web::scope("/riderr").service(services::riderr::routes::ride_stats)
}
