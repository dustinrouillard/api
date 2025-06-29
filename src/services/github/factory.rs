use actix_web::{web, Scope};

use crate::services;

pub fn factory() -> Scope {
  web::scope("/github")
    .service(services::github::routes::github_pinned)
    .service(services::github::routes::github_contributions)
}
