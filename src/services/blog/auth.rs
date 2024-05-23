use crate::{
  connectivity::prisma::blog_admin_users,
  structs::blog::{
    BlogAdminIntSession, BlogLoginRequest, BlogUserMutate,
    BlogUserPasswordChange,
  },
  ServerState,
};

use argon2::{self, Config};
use rand::{distributions::Alphanumeric, Rng};

use actix_web::{
  delete, get,
  http::Error,
  patch, post,
  web::{self, Json},
  HttpMessage, HttpRequest, HttpResponse,
};
use redis::aio::ConnectionManager;
use serde_json::json;

#[post("/auth")]
async fn login(
  body: web::Json<BlogLoginRequest>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let username = body.username.to_string();

  let prisma = &state.prisma;
  let valkey = &mut state.valkey.clone();

  let user_lookup = prisma
    .blog_admin_users()
    .find_first(vec![blog_admin_users::username::equals(username)])
    .exec()
    .await;

  match user_lookup {
    Ok(user) => match user {
      Some(user) => {
        let password = body.password.as_bytes();
        let valid =
          argon2::verify_encoded(&user.password, password).unwrap();

        if !valid {
          return Ok(
            HttpResponse::Unauthorized()
              .append_header(("Content-type", "application/json"))
              .body(json!({"code": "invalid_authentication"}).to_string()),
          );
        }

        let session_token: String = rand::thread_rng()
          .sample_iter(&Alphanumeric)
          .take(96)
          .map(char::from)
          .collect();

        let _ = redis::cmd("SET")
          .arg(format!("blog_admin_session/{}", session_token))
          .arg(json!({"user_id": user.id}).to_string())
          .query_async::<ConnectionManager, String>(&mut valkey.cm)
          .await;

        let response = json!({ "user": { "id": user.id, "username": user.username, "display_name": user.display_name, }, "session": { "token": session_token } });

        Ok(
          HttpResponse::Ok()
            .append_header(("Content-type", "application/json"))
            .body(response.to_string()),
        )
      }
      None => Ok(
        HttpResponse::Unauthorized()
          .append_header(("Content-type", "application/json"))
          .body(json!({"code": "invalid_username"}).to_string()),
      ),
    },
    Err(_) => Ok(
      HttpResponse::Unauthorized()
        .append_header(("Content-type", "application/json"))
        .body(json!({"code": "failed_to_lookup_user"}).to_string()),
    ),
  }
}

#[get("/me")]
async fn get_user(req: HttpRequest) -> Result<HttpResponse, Error> {
  let exts = req.extensions_mut();
  let user = exts.get::<blog_admin_users::Data>().unwrap();

  Ok(
    HttpResponse::Ok()
      .append_header(("Content-type", "application/json"))
      .body(
        json!({
            "user": {
                "id": user.id,
                "username": user.username,
                "display_name": user.display_name,
            }
        })
        .to_string(),
      ),
  )
}

#[patch("/me")]
async fn update_user(
  req: HttpRequest,
  state: web::Data<ServerState>,
  body: Option<web::Json<BlogUserMutate>>,
) -> Result<HttpResponse, Error> {
  let exts = req.extensions_mut();
  let user = exts.get::<blog_admin_users::Data>().unwrap();
  let prisma = &state.prisma;

  let body: Json<BlogUserMutate> =
    body.unwrap_or(actix_web::web::Json(BlogUserMutate {
      username: None,
      display_name: None,
    }));

  let update_params: Vec<blog_admin_users::SetParam> = vec![
    body.username.clone().map(blog_admin_users::username::set),
    body.display_name.clone().map(|value: std::string::String| {
      blog_admin_users::display_name::set(Some(value))
    }),
  ]
  .into_iter()
  .flatten()
  .collect();

  let user_record = prisma
    .blog_admin_users()
    .update(
      blog_admin_users::id::equals(user.id.to_string()),
      update_params,
    )
    .exec()
    .await
    .unwrap();

  Ok(
    HttpResponse::Ok()
      .append_header(("Content-type", "application/json"))
      .body(
        json!({
            "user": {
                "id": user_record.id,
                "username": user_record.username,
                "display_name": user_record.display_name,
            }
        })
        .to_string(),
      ),
  )
}

#[patch("/auth")]
async fn change_password(
  req: HttpRequest,
  state: web::Data<ServerState>,
  body: web::Json<BlogUserPasswordChange>,
) -> Result<HttpResponse, Error> {
  let exts = req.extensions_mut();
  let user = exts.get::<blog_admin_users::Data>().unwrap();

  let prisma = &state.prisma;

  if body.password == body.new_password {
    return Ok(
      HttpResponse::BadRequest()
        .append_header(("Content-type", "application/json"))
        .body(json!({"code": "old_and_new_passwords_match"}).to_string()),
    );
  }

  let password = body.password.as_bytes();
  let valid = argon2::verify_encoded(&user.password, password).unwrap();

  if !valid {
    return Ok(
      HttpResponse::Unauthorized()
        .append_header(("Content-type", "application/json"))
        .body(json!({"code": "current_password_invalid"}).to_string()),
    );
  }

  let password_salt: String = rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(48)
    .map(char::from)
    .collect();

  let config = Config::default();
  let new_hash = argon2::hash_encoded(
    body.new_password.as_bytes(),
    password_salt.as_bytes(),
    &config,
  )
  .unwrap();

  let _ = prisma
    .blog_admin_users()
    .update(
      blog_admin_users::id::equals(user.id.to_string()),
      vec![blog_admin_users::password::set(new_hash)],
    )
    .exec()
    .await
    .unwrap();

  Ok(HttpResponse::NoContent().finish())
}

#[delete("/auth")]
async fn logout(
  req: HttpRequest,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let valkey = &mut state.valkey.clone();

  let exts = req.extensions_mut();
  let session = exts.get::<BlogAdminIntSession>().unwrap();

  let _ = redis::cmd("DEL")
    .arg(format!("blog_admin_session/{}", session.token))
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await;

  Ok(HttpResponse::NoContent().finish())
}
