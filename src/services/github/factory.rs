use actix_web::{web, Scope};

use crate::services;

pub fn github_factory() -> Scope {
  web::scope("/github")
    .service(services::github::routes::github_pinned)
    .service(services::github::routes::github_contributions)
}
