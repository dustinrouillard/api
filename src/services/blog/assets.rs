use actix_multipart::form::MultipartForm;
use actix_web::{delete, get, http::Error, post, web, HttpResponse};
use serde_json::json;
use sha1::{Digest, Sha1};

use crate::{structs::blog::{BlogAsset, BlogAssetUpload}, ServerState};

#[post("/posts/{id}/assets")]
async fn upload_asset_for_post(
  MultipartForm(form): MultipartForm<BlogAssetUpload>,
  state: web::Data<ServerState>,
  post_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let s3 = &state.s3;

  let file = &mut form.files.first().unwrap();

  let type_match = crate::services::uploads::helpers::is_allowed_type(
    file,
    "files".to_string(),
  )
  .1;

  let mut hasher = Sha1::new();
  hasher.update(&file.data);
  let result = hasher.finalize();

  let hash = hex::encode(result);
  let ext = type_match.ext;
  let path = format!("blog/assets/{hash}.{ext}");

  let response = s3
    .cdn_bucket
    .put_object_with_content_type(&path, &file.data, &type_match.mime)
    .await
    .unwrap();

  if response.status_code() != 200 {
    return Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "failed_upload_to_s3"})),
    );
  }

  let size = file.data.len() as i32;

  let asset = sqlx::query_as::<_, BlogAsset>(
    "INSERT INTO blog_assets (hash, post_id, file_type, file_size) \
     VALUES ($1, $2, $3, $4) RETURNING *",
  )
  .bind(hash.clone())
  .bind(post_id.to_string())
  .bind(ext.clone())
  .bind(size)
  .fetch_one(&state.db)
  .await;

  match asset {
    Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "asset_already_exists"})),
    ),
    Err(_) => Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "failed_to_create_asset"})),
    ),
    Ok(asset) => Ok(HttpResponse::Ok().json(
      json!({"asset": { "hash": hash, "post_id": post_id.to_string(), "file_type": ext, "file_size": size, "upload_date": asset.upload_date }}),
    )),
  }
}

#[get("/posts/{id}/assets")]
async fn get_assets_for_post(
  post_id: web::Path<String>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let assets: Vec<BlogAsset> = sqlx::query_as::<_, BlogAsset>(
    "SELECT * FROM blog_assets WHERE post_id = $1 ORDER BY upload_date ASC",
  )
  .bind(post_id.to_string())
  .fetch_all(&state.db)
  .await
  .unwrap();

  let assets: Vec<serde_json::Value> = assets
    .iter()
    .map(|post| {
      json!({
        "hash": post.hash,
        "post_id": post.post_id,
        "file_type": post.file_type,
        "file_size": post.file_size,
        "upload_date": post.upload_date,
      })
    })
    .collect();

  Ok(HttpResponse::Ok().json(json!({"assets": assets})))
}

#[delete("/posts/{id}/assets/{hash}")]
async fn delete_asset_for_post(
  state: web::Data<ServerState>,
  params: web::Path<Vec<String>>,
) -> Result<HttpResponse, Error> {
  let post_id = params.first().unwrap();
  let hash = params.last().unwrap();

  let s3 = &state.s3;

  let asset = sqlx::query_as::<_, BlogAsset>(
    "SELECT * FROM blog_assets WHERE hash = $1 AND post_id = $2 LIMIT 1",
  )
  .bind(hash.to_string())
  .bind(post_id.to_string())
  .fetch_optional(&state.db)
  .await;

  match asset {
    Ok(Some(asset)) => {
      let file_type = asset.file_type;

      let res = s3
        .cdn_bucket
        .delete_object(format!("blog/assets/{hash}.{file_type}"))
        .await;

      if res.unwrap().status_code() != 204 {
        return Ok(HttpResponse::BadRequest().json(json!({
          "code": "failed_to_delete_from_s3"
        })));
      }

      let _ = sqlx::query("DELETE FROM blog_assets WHERE hash = $1")
        .bind(hash.to_string())
        .execute(&state.db)
        .await;

      Ok(HttpResponse::NoContent().finish())
    }
    Ok(None) => Ok(HttpResponse::NotFound().json(json!({
      "code": "asset_not_found"
    }))),
    Err(_) => Ok(HttpResponse::BadRequest().json(json!({
      "code": "error_with_asset_lookup"
    }))),
  }
}
