use serde::Deserialize;
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
