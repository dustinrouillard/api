use std::io::Error;

use envconfig::Envconfig;

use crate::{config::Config, structs::weather::UpstreamWeather};

pub async fn fetch_weather_data(
  coords: Option<String>,
) -> Result<UpstreamWeather, Error> {
  let config = Config::init_from_env().unwrap();

  let client = reqwest::Client::new();
  let res = client
    .get(format!(
      "https://weather.dstn.to/coords/{}",
      coords.unwrap_or(config.weather_coords)
    ))
    .send()
    .await
    .unwrap();

  let status = res.status().as_u16();

  if status != 200 {
    return Err(Error::new(
      std::io::ErrorKind::Other,
      "weather query failed",
    ));
  }

  let body = res.json::<UpstreamWeather>().await.unwrap();

  Ok(body)
}
