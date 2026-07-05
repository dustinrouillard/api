use crate::{
  structs::blog::{
    BlogAdminIntSession, BlogAdminUser, BlogLoginRequest, BlogUserMutate,
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

  let valkey = &mut state.valkey.clone();

  let user_lookup = sqlx::query_as::<_, BlogAdminUser>(
    "SELECT * FROM blog_admin_users WHERE username = $1 LIMIT 1",
  )
  .bind(username)
  .fetch_optional(&state.db)
  .await;

  match user_lookup {
    Ok(Some(user)) => {
      let password = body.password.as_bytes();
      let valid = argon2::verify_encoded(&user.password, password).unwrap();

      if !valid {
        return Ok(
          HttpResponse::Unauthorized()
            .json(json!({"code": "invalid_authentication"})),
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

      Ok(HttpResponse::Ok().json(response))
    }
    Ok(None) => Ok(
      HttpResponse::Unauthorized()
        .json(json!({"code": "invalid_username"})),
    ),
    Err(_) => Ok(
      HttpResponse::Unauthorized()
        .json(json!({"code": "failed_to_lookup_user"})),
    ),
  }
}

#[get("/me")]
async fn get_user(req: HttpRequest) -> Result<HttpResponse, Error> {
  let exts = req.extensions_mut();
  let user = exts.get::<BlogAdminUser>().unwrap();

  Ok(HttpResponse::Ok().json(json!({
      "user": {
          "id": user.id,
          "username": user.username,
          "display_name": user.display_name,
      }
  })))
}

#[patch("/me")]
async fn update_user(
  req: HttpRequest,
  state: web::Data<ServerState>,
  body: Option<web::Json<BlogUserMutate>>,
) -> Result<HttpResponse, Error> {
  let exts = req.extensions_mut();
  let user = exts.get::<BlogAdminUser>().unwrap();

  let body: Json<BlogUserMutate> =
    body.unwrap_or(actix_web::web::Json(BlogUserMutate {
      username: None,
      display_name: None,
    }));

  let username = body.username.clone().unwrap_or(user.username.clone());
  let display_name =
    body.display_name.clone().or(user.display_name.clone());

  let user_record = sqlx::query_as::<_, BlogAdminUser>(
    "UPDATE blog_admin_users SET username = $1, display_name = $2 \
     WHERE id = $3 RETURNING *",
  )
  .bind(username)
  .bind(display_name)
  .bind(user.id.to_string())
  .fetch_one(&state.db)
  .await
  .unwrap();

  Ok(HttpResponse::Ok().json(json!({
      "user": {
          "id": user_record.id,
          "username": user_record.username,
          "display_name": user_record.display_name,
      }
  })))
}

#[patch("/auth")]
async fn change_password(
  req: HttpRequest,
  state: web::Data<ServerState>,
  body: web::Json<BlogUserPasswordChange>,
) -> Result<HttpResponse, Error> {
  let exts = req.extensions_mut();
  let user = exts.get::<BlogAdminUser>().unwrap();

  if body.password == body.new_password {
    return Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "old_and_new_passwords_match"})),
    );
  }

  let password = body.password.as_bytes();
  let valid = argon2::verify_encoded(&user.password, password).unwrap();

  if !valid {
    return Ok(
      HttpResponse::Unauthorized()
        .json(json!({"code": "current_password_invalid"})),
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

  let _ = sqlx::query(
    "UPDATE blog_admin_users SET password = $1 WHERE id = $2",
  )
  .bind(new_hash)
  .bind(user.id.to_string())
  .execute(&state.db)
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
