use actix_multipart::form::{bytes::Bytes, MultipartForm};

#[derive(Debug, MultipartForm)]
pub struct CdnUpload {
  #[multipart(rename = "file")]
  pub files: Vec<Bytes>,
}
