use envconfig::Envconfig;

#[derive(Envconfig)]
pub struct Config {
  #[envconfig(from = "ENV", default = "dev")]
  pub env: String,

  #[envconfig(from = "LISTEN_HOST", default = "0.0.0.0")]
  pub listen_host: String,

  #[envconfig(from = "LISTEN_PORT", default = "8080")]
  pub listen_port: u16,

  #[envconfig(
    from = "SPOTIFY_CLIENT_ID",
    default = "01ba26764aca4594a26f4cc59cd3f01f"
  )]
  pub spotify_client_id: String,

  #[envconfig(from = "SPOTIFY_CLIENT_SECRET")]
  pub spotify_client_secret: String,

  #[envconfig(
    from = "SPOTIFY_REDIRECT_URI",
    default = "http://localhost:8080/v2/spotify/setup"
  )]
  pub spotify_redirect_uri: String,

  #[envconfig(from = "VALKEY_DSN", default = "redis://valkey:6379")]
  pub valkey_dsn: String,

  #[envconfig(
    from = "RABBIT_DSN",
    default = "amqp://rabbit:password@rabbitmq:5672"
  )]
  pub rabbit_dsn: String,

  #[envconfig(
    from = "RABBIT_QUEUE_NAME",
    default = "dstn-gateway-ingest"
  )]
  pub rabbit_queue: String,

  #[envconfig(from = "S3_ENDPOINT", default = "https://s3.kush")]
  pub s3_endpoint: String,

  #[envconfig(from = "S3_CLIENT_ID", default = "")]
  pub s3_client_id: String,

  #[envconfig(from = "S3_CLIENT_SECRET", default = "")]
  pub s3_client_secret: String,

  #[envconfig(from = "S3_BUCKET_NAME", default = "cdn")]
  pub s3_bucket_name: String,

  #[envconfig(from = "S3_REGION", default = "none")]
  pub s3_region: String,

  #[envconfig(from = "S3_IMAGES_ALIAS", default = "dustin.pics")]
  pub s3_images_alias: String,

  #[envconfig(from = "S3_FILES_ALIAS", default = "files.dstn.to")]
  pub s3_files_alias: String,
}
