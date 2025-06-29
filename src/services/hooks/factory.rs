use actix_web::{web, Scope};

use crate::services;

pub fn factory() -> Scope {
  web::scope("/hooks").service(services::hooks::riderr::execute)
}
