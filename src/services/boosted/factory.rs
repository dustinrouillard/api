use actix_web::{web, Scope};

use crate::services;

pub fn boosted_factory() -> Scope {
  web::scope("/boosted").service(services::boosted::routes::ride_stats)
}
