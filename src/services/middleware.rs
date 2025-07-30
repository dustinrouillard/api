use actix_web::{
  body::MessageBody,
  dev::{ServiceRequest, ServiceResponse},
  http::header,
  web::Data,
  Error, HttpResponse,
};
use actix_web_lab::middleware::Next;
use serde_json::json;

use crate::{helpers::authentication::is_management_authed, ServerState};

pub async fn auth_middleware(
  req: ServiceRequest,
  next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
  let auth_token = req.headers().get(header::AUTHORIZATION);

  let state = req.app_data::<Data<ServerState>>().unwrap().clone();

  let valkey = &mut state.valkey.clone();

  match auth_token {
    None => {
      return Ok(
        ServiceResponse::new(
          req.request().to_owned(),
          HttpResponse::Unauthorized()
            .json(json!({"code": "missing_authentication"})),
        )
        .map_into_boxed_body(),
      );
    }
    Some(token) => {
      let is_management_authed =
        is_management_authed(valkey, Some(token)).await;

      if let Ok(authed) = is_management_authed {
        if !authed {
          return Ok(
            ServiceResponse::new(
              req.request().to_owned(),
              HttpResponse::Unauthorized()
                .json(json!({"code": "invalid_authentication"})),
            )
            .map_into_boxed_body(),
          );
        }
      } else {
        return Ok(
          ServiceResponse::new(
            req.request().to_owned(),
            HttpResponse::Unauthorized()
              .json(json!({"code": "invalid_authentication"})),
          )
          .map_into_boxed_body(),
        );
      }

      return next
        .call(req)
        .await
        .map(ServiceResponse::map_into_boxed_body);
    }
  }
}
