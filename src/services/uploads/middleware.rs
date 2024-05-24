use actix_web::{
  body::MessageBody,
  dev::{ServiceRequest, ServiceResponse},
  http::header,
  web::Data,
  Error, HttpResponse,
};
use actix_web_lab::middleware::Next;
use redis::aio::ConnectionManager;
use serde_json::json;

use crate::ServerState;

pub async fn uploads_auth_mw(
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
            .append_header(("Content-type", "application/json"))
            .body(json!({"code": "missing_authentication"}).to_string()),
        )
        .map_into_boxed_body(),
      );
    }
    Some(token) => {
      let valkey_session = redis::cmd("GET")
        .arg(format!("upload_tokens/{}", token.to_str().unwrap()))
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      match valkey_session {
        Err(_) => {
          return Ok(
            ServiceResponse::new(
              req.request().to_owned(),
              HttpResponse::Unauthorized()
                .append_header(("Content-type", "application/json"))
                .body(
                  json!({"code": "invalid_authentication"}).to_string(),
                ),
            )
            .map_into_boxed_body(),
          );
        }

        Ok(_) => {
          return next
            .call(req)
            .await
            .map(ServiceResponse::map_into_boxed_body);
        }
      }
    }
  }
}
