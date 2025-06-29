pub mod routes;

use actix_web::{web, Scope};

pub fn factory() -> Scope {
  web::scope("/ig").service(routes::get_overview)
}
