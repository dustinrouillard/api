use actix_web::{body::MessageBody, dev::ServiceFactory, web, Scope};
use actix_web_lab::middleware::from_fn;

use crate::services;

pub fn factory() -> Scope<
  impl ServiceFactory<
    actix_web::dev::ServiceRequest,
    Config = (),
    Response = actix_web::dev::ServiceResponse<impl MessageBody>,
    Error = actix_web::Error,
    InitError = (),
  >,
> {
  web::scope("/uploads")
    .wrap(from_fn(services::uploads::middleware::uploads_auth_mw))
    .service(services::uploads::routes::upload_to_cdn)
}
