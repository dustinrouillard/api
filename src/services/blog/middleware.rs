use actix_web::{
  body::MessageBody,
  dev::{ServiceRequest, ServiceResponse},
  http::header,
  web::Data,
  Error, HttpMessage, HttpResponse,
};
use actix_web_lab::middleware::Next;
use redis::aio::ConnectionManager;
use serde_json::json;

use crate::{
  connectivity::prisma::blog_admin_users,
  structs::blog::{BlogAdminIntSession, BlogAdminSession},
  ServerState,
};

pub async fn blog_admin_auth_mw(
  req: ServiceRequest,
  next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
  let auth_token = req.headers().get(header::AUTHORIZATION);

  let state = req.app_data::<Data<ServerState>>().unwrap().clone();

  let prisma = &state.prisma;
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
      let redis_session = redis::cmd("GET")
        .arg(format!("blog_admin_session/{}", token.to_str().unwrap()))
        .query_async::<ConnectionManager, String>(&mut valkey.cm)
        .await;

      match redis_session {
        Err(_) => {
          return Ok(
            ServiceResponse::new(
              req.request().to_owned(),
              HttpResponse::Unauthorized()
                .json(json!({"code": "invalid_authentication_state"})),
            )
            .map_into_boxed_body(),
          );
        }

        Ok(session_token) => {
          let session =
            serde_json::from_str::<BlogAdminSession>(&session_token)
              .unwrap();

          let user_record = prisma
            .blog_admin_users()
            .find_first(vec![blog_admin_users::id::equals(
              session.user_id,
            )])
            .exec()
            .await;

          match user_record {
            Ok(user) => match user {
              Some(user) => {
                req.extensions_mut().insert(user.clone());
                req.extensions_mut().insert(BlogAdminIntSession {
                  user_id: user.id.to_string(),
                  token: token.to_str().unwrap().to_string(),
                });

                return next
                  .call(req)
                  .await
                  .map(ServiceResponse::map_into_boxed_body);
              }
              None => {
                return Ok(
                  ServiceResponse::new(
                    req.request().to_owned(),
                    HttpResponse::Unauthorized().json(
                      json!({"code": "invalid_authentication_user"}),
                    ),
                  )
                  .map_into_boxed_body(),
                )
              }
            },
            Err(_) => {
              return Ok(
                ServiceResponse::new(
                  req.request().to_owned(),
                  HttpResponse::Unauthorized()
                    .json(json!({"code": "invalid_authentication_user"})),
                )
                .map_into_boxed_body(),
              );
            }
          }
        }
      }
    }
  }
}
