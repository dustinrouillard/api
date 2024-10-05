use actix_web::{web, Scope};

use crate::services;

pub fn hooks_factory() -> Scope {
  web::scope("/hooks").service(services::hooks::boosted::execute)
}
