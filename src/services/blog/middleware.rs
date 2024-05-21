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
    structs::blog::{BlogAdminIntSession, BlogAdminSession, BlogAdminUser},
    ServerState,
};

pub async fn blog_admin_auth_mw(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let auth_token = req.headers().get(header::AUTHORIZATION);

    let state = req.app_data::<Data<ServerState>>().unwrap().clone();

    let redis = &mut state.valkey.clone();
    let postgres = &mut state.postgres.clone();

    match auth_token {
        None => {
            return Ok(ServiceResponse::new(
                req.request().to_owned(),
                HttpResponse::Unauthorized()
                    .append_header(("Content-type", "application/json"))
                    .body(json!({"code": "missing_authentication"}).to_string()),
            )
            .map_into_boxed_body());
        }
        Some(token) => {
            let redis_session = redis::cmd("GET")
                .arg(format!("blog_admin_session/{}", token.to_str().unwrap()))
                .query_async::<ConnectionManager, String>(&mut redis.cm)
                .await;

            match redis_session {
                Err(_) => {
                    return Ok(ServiceResponse::new(
                        req.request().to_owned(),
                        HttpResponse::Unauthorized()
                            .append_header(("Content-type", "application/json"))
                            .body(json!({"code": "invalid_authentication_state"}).to_string()),
                    )
                    .map_into_boxed_body());
                }

                Ok(session_token) => {
                    let session = serde_json::from_str::<BlogAdminSession>(&session_token).unwrap();

                    let user = sqlx::query_as::<_, BlogAdminUser>(
                        "SELECT id, username, display_name FROM blog_admin_users WHERE id = $1;",
                    )
                    .bind(session.user_id)
                    .fetch_one(&postgres.pool)
                    .await;

                    match user {
                        Ok(user) => {
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
                        Err(_) => {
                            return Ok(ServiceResponse::new(
                                req.request().to_owned(),
                                HttpResponse::Unauthorized()
                                    .append_header(("Content-type", "application/json"))
                                    .body(
                                        json!({"code": "invalid_authentication_user"}).to_string(),
                                    ),
                            )
                            .map_into_boxed_body());
                        }
                    }
                }
            }
        }
    }
}
