use actix_web::{web, Scope};
use actix_web_lab::middleware::from_fn;

use crate::services;

pub fn blog_factory() -> Scope {
  web::scope("/blog")
    .service(services::blog::auth::login)
    .service(services::blog::posts::get_post)
    .service(services::blog::posts::get_posts)
    .service(
      web::scope("/admin")
        .wrap(from_fn(services::blog::middleware::blog_admin_auth_mw))
        .service(services::blog::auth::logout)
        .service(services::blog::auth::get_user)
        .service(services::blog::auth::update_user)
        .service(services::blog::auth::change_password)
        .service(services::blog::posts::get_all_posts)
        .service(services::blog::posts::create_post)
        .service(services::blog::posts::update_post)
        .service(services::blog::posts::delete_post)
        .service(services::blog::assets::get_assets_for_post)
        .service(services::blog::assets::upload_asset_for_post)
        .service(services::blog::assets::delete_asset_for_post),
    )
}
