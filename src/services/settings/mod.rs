pub mod routes;
pub mod structs;

use actix_web::{web, Scope};

pub fn factory() -> Scope {
  web::scope("/settings").service(routes::get_site_settings)
}
