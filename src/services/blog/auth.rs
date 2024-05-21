use crate::{
    structs::blog::{BlogAdminIntSession, BlogAdminUser, BlogLoginRequest},
    ServerState,
};

use argon2::{self};
use rand::{distributions::Alphanumeric, Rng};

use actix_web::{delete, get, http::Error, post, web, HttpMessage, HttpRequest, HttpResponse};
use redis::aio::ConnectionManager;
use serde_json::json;

#[post("/auth")]
async fn login(
    body: web::Json<BlogLoginRequest>,
    state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
    let username = body.username.to_string();

    let redis = &mut state.valkey.clone();
    let postgres = &mut state.postgres.clone();

    let user_record = sqlx::query_as::<_, BlogAdminUser>(
        "SELECT * FROM blog_admin_users WHERE username = $1 LIMIT 1;",
    )
    .bind(username)
    .fetch_one(&postgres.pool)
    .await
    .map_err(|e| format!("{}", e));

    // if user_record.is_err() {
    //     return Err(Error);
    // }

    let user = user_record.unwrap();

    let password = body.password.as_bytes();
    let valid = argon2::verify_encoded(&user.password.unwrap(), password).unwrap();

    if !valid {
        let json = json!({"code": "invalid_authentication"});

        return Ok(HttpResponse::Unauthorized()
            .append_header(("Content-type", "application/json"))
            .body(json.to_string()));
    }

    let session_token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(96)
        .map(char::from)
        .collect();

    let _ = redis::cmd("SET")
        .arg(format!("blog_admin_session/{}", session_token))
        .arg(json!({"user_id": user.id}).to_string())
        .query_async::<ConnectionManager, String>(&mut redis.cm)
        .await;

    let json = json!({ "user": { "id": user.id, "username": user.username, "display_name": user.display_name, }, "session": { "token": session_token } });

    Ok(HttpResponse::Ok()
        .append_header(("Content-type", "application/json"))
        .body(json.to_string()))
}

#[get("/me")]
async fn get_user(req: HttpRequest) -> Result<HttpResponse, Error> {
    let exts = req.extensions_mut();
    let user = exts.get::<BlogAdminUser>().unwrap();

    Ok(HttpResponse::Ok()
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
        ))
}

#[delete("/auth")]
async fn logout(req: HttpRequest, state: web::Data<ServerState>) -> Result<HttpResponse, Error> {
    // let state = state.get_mut();
    let redis = &mut state.valkey.clone();

    let exts = req.extensions_mut();
    let session = exts.get::<BlogAdminIntSession>().unwrap();

    println!("{}", session.token);
    println!("{}", session.user_id);

    let _ = redis::cmd("DEL")
        .arg(format!("blog_admin_session/{}", session.token))
        .query_async::<ConnectionManager, String>(&mut redis.cm)
        .await;

    Ok(HttpResponse::NoContent().finish())
}
