use actix_multipart::form::MultipartForm;
use actix_web::{delete, get, http::Error, post, web, HttpResponse};
use prisma_client_rust::{
  prisma_errors::query_engine::UniqueKeyViolation, Direction,
};
use serde_json::json;
use sha1::{Digest, Sha1};

use crate::{
  connectivity::prisma::blog_assets, structs::blog::BlogAssetUpload,
  ServerState,
};

#[post("/posts/{id}/assets")]
async fn upload_asset_for_post(
  MultipartForm(form): MultipartForm<BlogAssetUpload>,
  state: web::Data<ServerState>,
  post_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
  let s3 = &state.s3;
  let prisma = &state.prisma;

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

  let asset = prisma
    .blog_assets()
    .create(
      hash.clone(),
      ext.clone(),
      crate::connectivity::prisma::blog_posts::UniqueWhereParam::IdEquals(
        post_id.to_string(),
      ),
      vec![blog_assets::file_size::set(size)],
    )
    .exec()
    .await;

  match asset {
    Err(error) if error.is_prisma_error::<UniqueKeyViolation>() => Ok(
        HttpResponse::BadRequest()
        .json(json!({"code": "asset_already_exists"})),
    ),
    Err(_) => {
      Ok(
        HttpResponse::BadRequest()
          .json(json!({"code": "failed_to_create_asset"})),
      )
    },
    Ok(asset) => Ok(
        HttpResponse::Ok()
          .json(
            json!({"asset": { "hash": hash, "post_id": post_id.to_string(), "file_type": ext, "file_size": size, "upload_date": asset.upload_date }}),
          ),
      ),
  }
}

#[get("/posts/{id}/assets")]
async fn get_assets_for_post(
  post_id: web::Path<String>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let prisma = &mut &state.prisma;

  let assets: Vec<blog_assets::Data> = prisma
    .blog_assets()
    .find_many(vec![blog_assets::post_id::equals(post_id.to_string())])
    .order_by(blog_assets::upload_date::order(Direction::Asc))
    .exec()
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

  let prisma = &mut &state.prisma;
  let s3 = &state.s3;

  let asset = prisma
    .blog_assets()
    .find_first(vec![
      blog_assets::hash::equals(hash.to_string()),
      blog_assets::post_id::equals(post_id.to_string()),
    ])
    .exec()
    .await;

  match asset {
    Ok(asset) => match asset {
      Some(asset) => {
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

        let _ = prisma
          .blog_assets()
          .delete(blog_assets::hash::equals(hash.to_string()))
          .exec()
          .await;

        Ok(HttpResponse::NoContent().finish())
      }
      None => Ok(HttpResponse::NotFound().json(json!({
        "code": "asset_not_found"
      }))),
    },
    Err(_) => Ok(HttpResponse::BadRequest().json(json!({
      "code": "error_with_asset_lookup"
    }))),
  }
}
