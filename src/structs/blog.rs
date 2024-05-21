use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sqlx::FromRow;
extern crate serde_json;

#[derive(Deserialize)]
pub struct BlogLoginRequest {
    pub username: String,
    pub password: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct BlogAdminSession {
    pub user_id: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct BlogAdminIntSession {
    pub user_id: String,
    pub token: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct BlogAdminUser {
    pub id: String,
    pub username: String,
    #[sqlx(default)]
    pub display_name: Option<String>,
    #[sqlx(default)]
    pub password: Option<String>,
}

#[serde_as]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct BlogPost {
    pub id: String,
    pub slug: String,
    pub title: String,
    #[sqlx(default)]
    pub description: Option<String>,
    #[sqlx(default)]
    pub image: Option<String>,
    pub visibility: String,
    #[sqlx(default)]
    pub tags: Option<Vec<String>>,
    #[sqlx(default)]
    pub body: Option<String>,
    pub created_at: NaiveDateTime,
    #[sqlx(default)]
    pub published_at: Option<NaiveDateTime>,
}
