use actix_multipart::form::MultipartForm;
use actix_web::{http::Error, post, web, HttpResponse};
use envconfig::Envconfig;
use serde_json::json;
use sha1::{Digest, Sha1};

use crate::{
  config::Config, services::uploads::helpers, structs::uploads::CdnUpload,
  ServerState,
};

#[post("{type}")]
async fn upload_to_cdn(
  MultipartForm(form): MultipartForm<CdnUpload>,
  state: web::Data<ServerState>,
  asset_type: web::Path<String>,
) -> Result<HttpResponse, Error> {
  if asset_type.to_string() != "images"
    && asset_type.to_string() != "files"
  {
    return Ok(
      HttpResponse::BadRequest()
        .append_header(("Content-type", "application/json"))
        .body(json!({"code": "invalid_asset_type"}).to_string()),
    );
  }

  let file = &mut form.files.first().unwrap();

  let type_match: helpers::AssetType =
    match helpers::is_allowed_type(file, asset_type.to_string()) {
      (true, res) => res,
      (false, res) => {
        if asset_type.to_string() == "images" {
          return Ok(
            HttpResponse::BadRequest()
              .append_header(("Content-type", "application/json"))
              .body(json!({"code": "prohibited_file_type"}).to_string()),
          );
        }

        res
      }
    };

  let base_dir = if asset_type.to_string() == "images" {
    "i"
  } else {
    "u"
  };

  let mut hasher = Sha1::new();
  hasher.update(&file.data);
  let result = hasher.finalize();

  let hash = hex::encode(result);
  let hash = &hash[..16].to_string();
  let ext = type_match.ext;

  let path = format!("{base_dir}/{hash}.{ext}");

  let s3 = &state.s3;
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

  let config = Config::init_from_env().unwrap();
  let alias_url = if asset_type.to_string() == "images" {
    config.s3_images_alias
  } else {
    config.s3_files_alias
  };

  Ok(
    HttpResponse::Ok()
      .append_header(("Content-type", "application/json"))
      .body(json!({"data": { "url": format!("https://{alias_url}/{hash}.{ext}") }}).to_string()),
  )
}
