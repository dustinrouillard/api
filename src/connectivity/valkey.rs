use std::io::Error;

use envconfig::Envconfig;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{config::Config, modules::spotify::CurrentPlaying, services::spotify::SpotifyTokens};

#[derive(Clone)]
pub struct ValkeyManager {
    pub cm: ConnectionManager,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SpotifyAccount {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizationData {
    refresh_token: String,
    grant_type: String,
    redirect_uri: String,
}

impl ValkeyManager {
    pub async fn new() -> Self {
        let config = Config::init_from_env().unwrap();
        let client = Client::open(config.valkey_dsn).unwrap();
        let cm = ConnectionManager::new(client).await.unwrap();
        tracing::info!("Connected to valkey");
        Self { cm }
    }

    pub async fn check_spotify_setup(&mut self) -> bool {
        let res: bool = self.cm.exists("spotify/refresh_token").await.unwrap();
        return res;
    }

    pub async fn get_spotify_account(&mut self) -> Result<SpotifyAccount, Error> {
        let access_token = self.cm.get("spotify/access_token").await;
        let refresh_token = self.cm.get("spotify/refresh_token").await.unwrap();

        if refresh_token == None {
            return Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "refresh_token missing",
            ));
        }

        match access_token {
            Ok(access_token) => {
                return Ok(SpotifyAccount {
                    access_token,
                    refresh_token,
                });
            }
            Err(..) => {
                let config = Config::init_from_env().unwrap();

                let redirect_uri = "http://127.0.0.1:8080/v2/spotify/setup";
                let data = AuthorizationData {
                    refresh_token: refresh_token.unwrap(),
                    grant_type: "refresh_token".into(),
                    redirect_uri: redirect_uri.into(),
                };

                let data = serde_urlencoded::to_string(&data)
                    .expect("error serializing data for spotify token");

                let client = reqwest::Client::new();
                let res = client
                    .post(format!("https://accounts.spotify.com/api/token?{}", data))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("Content-Length", "0")
                    .basic_auth(config.spotify_client_id, Some(config.spotify_client_secret))
                    .send()
                    .await
                    .unwrap();

                let status = res.status();

                if status.as_u16() == 200 {
                    let body = res.json::<SpotifyTokens>().await.unwrap();

                    self.save_spotify_account(
                        &body.access_token,
                        &body.refresh_token,
                        &body.expires_in,
                    )
                    .await;

                    return Ok(SpotifyAccount {
                        access_token: body.access_token,
                        refresh_token: body.refresh_token,
                    });
                } else {
                    tracing::debug!("Error regenerating spotify tokens");
                    return Ok(SpotifyAccount {
                        access_token: "".into(),
                        refresh_token: Some("".to_string()),
                    });
                }
            }
        }
    }

    pub async fn save_spotify_account(
        &mut self,
        access_token: &String,
        refresh_token: &Option<String>,
        expiry_ttl: &u32,
    ) {
        redis::cmd("SET")
            .arg("spotify/access_token")
            .arg(access_token)
            .arg("EX")
            .arg(expiry_ttl)
            .query_async::<ConnectionManager, String>(&mut self.cm)
            .await
            .unwrap();

        match refresh_token {
            Some(refresh_token) => {
                redis::cmd("SET")
                    .arg("spotify/refresh_token")
                    .arg(refresh_token)
                    .query_async::<ConnectionManager, String>(&mut self.cm)
                    .await
                    .unwrap();
            }
            None => (),
        }
    }

    pub async fn get_current(&mut self) -> CurrentPlaying {
        let current = redis::cmd("GET")
            .arg("spotify/current")
            .query_async::<ConnectionManager, String>(&mut self.cm)
            .await
            .unwrap();

        let json: CurrentPlaying = serde_json::from_str(&current).unwrap();

        return json;
    }

    pub async fn update_current(&mut self, data: &CurrentPlaying) {
        let json = serde_json::to_string(data).unwrap();

        redis::cmd("SET")
            .arg("spotify/current")
            .arg(json)
            .query_async::<ConnectionManager, String>(&mut self.cm)
            .await
            .unwrap();
    }

    pub async fn set_not_playing(&mut self) {
        let json = json!({"playing": false});

        redis::cmd("SET")
            .arg("spotify/current")
            .arg(json.to_string())
            .query_async::<ConnectionManager, String>(&mut self.cm)
            .await
            .unwrap();
    }
}
