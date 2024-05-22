use envconfig::Envconfig;

use s3::{creds::Credentials, error::S3Error, Bucket, Region};

use crate::config::Config;

#[warn(dead_code)]
#[derive(Clone)]
pub struct S3Manager {
  pub cdn_bucket: Bucket,
}

impl S3Manager {
  pub async fn new() -> Result<Self, S3Error> {
    let config = Config::init_from_env().unwrap();

    let region = Region::Custom {
      region: config.s3_region,
      endpoint: config.s3_endpoint,
    };
    let credentials = Credentials::new(
      Some(config.s3_client_id.as_str()),
      Some(config.s3_client_secret.as_str()),
      None,
      None,
      None,
    )
    .unwrap();

    let cdn_bucket = Bucket::new(
      &config.s3_bucket_name,
      region.clone(),
      credentials.clone(),
    )?
    .with_path_style();

    Ok(Self { cdn_bucket })
  }
}
