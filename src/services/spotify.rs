use actix_web::{get, http::Error, web, HttpResponse};
use envconfig::Envconfig;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{config::Config, ServerState};

#[derive(Debug, Deserialize, Clone)]
pub struct AuthorizeQuery {
    code: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthorizationData {
    code: String,
    grant_type: String,
    redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenError {
    pub error: String,
    pub error_description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyTokens {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u32,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[get("")]
async fn current(data: web::Data<ServerState>) -> Result<HttpResponse, Error> {
    let redis = &mut data.valkey.clone();

    let current = redis.get_current().await;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .body(json!({"success": true, "data": &current}).to_string()))
}

#[get("/authorize")]
async fn authorize(data: web::Data<ServerState>) -> Result<HttpResponse, Error> {
    let redis = &mut data.valkey.clone();

    let setup_check = redis.check_spotify_setup().await;
    if setup_check {
        tracing::debug!("spotify already setup");
        let json = json!({
            "code": "already_authorized",
            "message": "Spotify is already setup."
        });

        return Ok(HttpResponse::BadRequest()
            .insert_header(("Content-Type", "application/json"))
            .body(json.to_string()));
    }

    let config = Config::init_from_env().unwrap();

    let scope = "user-read-playback-state+user-read-currently-playing";
    let redirect_uri = "http://127.0.0.1:8080/spotify/setup";
    let url = format!("https://accounts.spotify.com/authorize?client_id={}&response_type=code&scope={}&redirect_uri={}", config.spotify_client_id, scope, redirect_uri);
    let json = json!({ "url": url });

    Ok(HttpResponse::Ok()
        .append_header(("Content-type", "application/json"))
        .body(json.to_string()))
}

#[get("/setup")]
async fn setup(
    data: web::Data<ServerState>,
    info: web::Query<AuthorizeQuery>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let redis = &mut data.valkey.clone();

    let setup_check = redis.check_spotify_setup().await;
    if setup_check {
        tracing::debug!("spotify already setup");
        let json = json!({
            "code": "already_authorized",
            "message": "Spotify is already setup."
        });

        return Ok(HttpResponse::BadRequest()
            .insert_header(("Content-Type", "application/json"))
            .body(json.to_string()));
    }

    let config = Config::init_from_env().unwrap();

    let code = &info.code;
    let redirect_uri = "http://127.0.0.1:8080/spotify/setup";
    let data = AuthorizationData {
        code: code.into(),
        grant_type: "authorization_code".into(),
        redirect_uri: redirect_uri.into(),
    };

    let data =
        serde_urlencoded::to_string(&data).expect("error serializing data for spotify token");

    let client = reqwest::Client::new();
    let res = client
        .post(format!("https://accounts.spotify.com/api/token?{}", data))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Content-Length", "0")
        .basic_auth(config.spotify_client_id, Some(config.spotify_client_secret))
        .send()
        .await?;

    let status = res.status();

    if status.as_u16() == 200 {
        let body = res.json::<SpotifyTokens>().await.unwrap();

        redis
            .save_spotify_account(&body.access_token, &body.refresh_token, &body.expires_in)
            .await;

        Ok(HttpResponse::NoContent().finish())
    } else {
        let body = res.json::<TokenError>().await.unwrap();
        Ok(HttpResponse::InternalServerError().json(body))
    }
}
