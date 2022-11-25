use envconfig::Envconfig;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::PgPoolOptions,
    types::{
        chrono::{DateTime, Utc},
        Json,
    },
    FromRow, Pool, Postgres,
};

use crate::config::Config;

#[derive(Clone)]
pub struct PostgresManager {
    pub pool: Pool<Postgres>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize)]
pub struct SpotifyArtist {
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, FromRow)]
pub struct SpotifyHistoryItem {
    pub id: String,
    #[sqlx(rename = "type")]
    pub spotify_history_type: String,
    pub name: String,
    pub artists: Vec<Json<SpotifyArtist>>,
    pub length: i32,
    pub image: String,
    pub device: i32,
    pub listened_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, FromRow)]
pub struct SpotifyDevice {
    pub id: i32,
    #[sqlx(rename = "type")]
    pub spotify_device_type: String,
    pub name: String,
}

impl PostgresManager {
    pub async fn new() -> PostgresManager {
        let config = Config::init_from_env().unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.postgres_dsn)
            .await
            .unwrap();

        tracing::info!("Connected to postgres");

        Self { pool }
    }

    pub async fn get_device_by_name(&mut self, name: String) -> Result<SpotifyDevice, sqlx::Error> {
        let item = sqlx::query_as::<_, SpotifyDevice>(
            "SELECT id, name, type FROM spotify_devices WHERE name = $1;",
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("{}", e));

        if item.is_err() {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(item.unwrap())
    }

    pub async fn insert_new_device(
        &mut self,
        name: String,
        device_type: String,
    ) -> Result<SpotifyDevice, sqlx::Error> {
        let item = sqlx::query_as::<_, SpotifyDevice>(
            "INSERT INTO spotify_devices (\"name\", \"type\") VALUES ($1, $2) RETURNING id, name, type;",
        )
        .bind(name)
        .bind(device_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("{}", e));

        if item.is_err() {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(item.unwrap())
    }

    pub async fn insert_new_history_item(&mut self, item: SpotifyHistoryItem) {
        sqlx::query(
            "INSERT INTO spotify_history (\"id\", \"type\", \"name\", \"length\", \"image\", \"listened_at\", \"device\", \"artists\") VALUES ($1, $2, $3, $4, $5, $6, $7, $8);",
        )
        .bind(item.id)
        .bind(item.spotify_history_type)
        .bind(item.name)
        .bind(item.length)
        .bind(item.image)
        .bind(item.listened_at)
        .bind(item.device)
        .bind(item.artists)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("{}", e)).unwrap();
    }

    pub async fn get_latest_history_item(
        &mut self,
        id: String,
    ) -> Result<SpotifyHistoryItem, sqlx::Error> {
        let item = sqlx::query_as::<_, SpotifyHistoryItem>(
            "SELECT id, type, name, artists, length, image, device, listened_at FROM spotify_history WHERE id = $1 ORDER BY listened_at DESC LIMIT 1;",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("{}", e));

        if item.is_err() {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(item.unwrap())
    }
}
