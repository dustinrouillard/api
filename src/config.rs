use envconfig::Envconfig;

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "LISTEN_HOST", default = "0.0.0.0")]
    pub listen_host: String,

    #[envconfig(from = "LISTEN_PORT", default = "8080")]
    pub listen_port: u16,

    #[envconfig(
        from = "SPOTIFY_CLIENT_ID",
        default = "01ba26764aca4594a26f4cc59cd3f01f"
    )]
    pub spotify_client_id: String,

    #[envconfig(
        from = "SPOTIFY_CLIENT_SECRET",
        default = "95dc3cd5184440b987996c2bf1876f76"
    )]
    pub spotify_client_secret: String,

    #[envconfig(
        from = "SPOTIFY_REDIRECT_URI",
        default = "http://localhost:8080/v1/spotify/setup"
    )]
    pub spotify_redirect_uri: String,

    #[envconfig(
        from = "POSTGRES_DSN",
        default = "postgres://postgres:postgres@10.8.0.7/testing"
    )]
    pub postgres_dsn: String,

    #[envconfig(from = "REDIS_DSN", default = "redis://10.7.20.3:6379")]
    pub redis_dsn: String,

    #[envconfig(from = "RABBIT_DSN", default = "amqp://rabbit:password@10.7.20.3:5672")]
    pub rabbit_dsn: String,

    #[envconfig(from = "RABBIT_QUEUE_NAME", default = "dstn-gateway-ingest")]
    pub rabbit_queue: String,
}
