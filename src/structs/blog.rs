use actix_multipart::form::{bytes::Bytes, MultipartForm};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sqlx::FromRow;
extern crate serde_json;

#[derive(Deserialize, Debug)]
pub struct BlogPostsQuery {
  pub limit: Option<i64>,
  pub offset: Option<i64>,
}

#[serde_as]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlogUserMutate {
  pub username: Option<String>,
  pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct BlogUserPasswordChange {
  pub password: String,
  pub new_password: String,
}

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
  pub display_name: Option<String>,
  pub password: String,
}

#[serde_as]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlogPostMutate {
  pub slug: Option<String>,
  pub title: Option<String>,
  pub description: Option<String>,
  pub image: Option<String>,
  pub visibility: Option<String>,
  pub tags: Option<Vec<String>>,
  pub body: Option<String>,
}

#[serde_as]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct BlogPost {
  pub id: String,
  pub slug: String,
  pub title: String,
  pub description: Option<String>,
  pub image: Option<String>,
  pub visibility: String,
  pub tags: Vec<String>,
  pub body: Option<String>,
  pub created_at: NaiveDateTime,
  pub published_at: Option<NaiveDateTime>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct BlogAsset {
  pub hash: String,
  pub post_id: String,
  pub file_type: String,
  pub file_size: i32,
  pub upload_date: NaiveDateTime,
}

#[derive(Debug, MultipartForm)]
pub struct BlogAssetUpload {
  #[multipart(rename = "file")]
  pub files: Vec<Bytes>,
}
