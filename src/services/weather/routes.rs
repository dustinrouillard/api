use actix_web::{get, http::Error, web, HttpResponse};
use redis::AsyncCommands;
use serde_json::json;

use crate::{services::weather::helper::fetch_weather_data, ServerState};

#[get("/current")]
async fn get_current_weather(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let redis = &mut state.valkey.clone();
  let override_cordnates =
    redis.cm.get::<_, String>("override/weather").await.ok();

  let weather = fetch_weather_data(override_cordnates).await;

  match weather {
    Ok(weather) => {
      let temperature = format!(
        "{:.0}",
        (weather.temperature.current - 273.15) * 9.0 / 5.0 + 32.0
      )
      .parse::<f64>()
      .unwrap();

      let humidity = weather.humidity;

      Ok(HttpResponse::Ok().json(json!(
        {
          "weather": {
            "temperature": temperature,
            "humidity": humidity
          }
        }
      )))
    }
    Err(_) => Ok(
      HttpResponse::BadRequest()
        .json(json!({"code": "weather_query_failed"})),
    ),
  }
}
