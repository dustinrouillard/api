use actix_multipart::form::MultipartForm;
use actix_web::{delete, get, http::Error, post, web, HttpResponse};
use prisma_client_rust::prisma_errors::query_engine::UniqueKeyViolation;
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
        .append_header(("Content-type", "application/json"))
        .body(json!({"code": "failed_upload_to_s3"}).to_string()),
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
        .append_header(("Content-type", "application/json"))
        .body(json!({"code": "asset_already_exists"}).to_string()),
    ),
    Err(_) => {
      Ok(
        HttpResponse::BadRequest()
          .append_header(("Content-type", "application/json"))
          .body(json!({"code": "failed_to_create_asset"}).to_string()),
      )
    },
    Ok(asset) => Ok(
        HttpResponse::Ok()
          .append_header(("Content-type", "application/json"))
          .body(
            json!({"asset": { "hash": hash, "post_id": post_id.to_string(), "file_type": ext, "file_size": size, "upload_date": asset.upload_date }})
              .to_string(),
          ),
      ),
  }
}

#[get("/posts/{id}/assets")]
async fn get_assets_for_post(
  post_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
  println!("{:?}", post_id);
  Ok(HttpResponse::NotImplemented().finish())
}

#[delete("/posts/{id}/assets/{hash}")]
async fn delete_asset_for_post(
  params: web::Path<Vec<String>>,
) -> Result<HttpResponse, Error> {
  println!("{:?}", params);
  Ok(HttpResponse::NotImplemented().finish())
}
