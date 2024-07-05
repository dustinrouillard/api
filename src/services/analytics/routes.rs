use actix_web::{get, http::Error, post, web, HttpRequest, HttpResponse};
use chrono::Utc;
use envconfig::Envconfig;
use futures::stream;
use influxdb2::{
  models::{DataPoint, Query},
  FromDataPoint,
};
use serde_json::json;

use crate::{config::Config, ServerState};

#[derive(Debug, FromDataPoint, Default)]
struct QueryResult {
  value: i64,
}

#[get("")]
async fn get_analytics(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let influxdb = &state.influxdb;
  let config = Config::init_from_env().unwrap();

  let day_query = Query::new(format!(
    "from(bucket: \"{}\")
      |> range(start: -24h)
      |> filter(fn: (r) => r._measurement == \"commands\")
      |> count()
    ",
    config.influxdb_bucket
  ));
  let day_result: Vec<QueryResult> = influxdb
    .client
    .query::<QueryResult>(Some(day_query))
    .await
    .unwrap();
  let day_count = day_result.first().map_or(0, |r| r.value);

  let week_query = Query::new(format!(
    "from(bucket: \"{}\")
      |> range(start: -7d)
      |> filter(fn: (r) => r._measurement == \"commands\")
      |> count()
    ",
    config.influxdb_bucket
  ));
  let week_result: Vec<QueryResult> = influxdb
    .client
    .query::<QueryResult>(Some(week_query))
    .await
    .unwrap();
  let week_count = week_result.first().map_or(0, |r| r.value);

  Ok(HttpResponse::Ok().json(json!(
    {
      "analytics": {
        "commands": {
          "day": day_count,
          "week": week_count
        },
      }
    }
  )))
}

#[post("/commands")]
async fn track_command(
  req: HttpRequest,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let influxdb = &state.influxdb;
  let config = Config::init_from_env().unwrap();

  let command_name = match req.headers().get("command-name") {
    Some(value) => value.to_str().unwrap().to_string(),
    None => "".to_string(),
  };

  let action_name = match req.headers().get("action-name") {
    Some(value) => value.to_str().unwrap().to_string(),
    None => "".to_string(),
  };

  let command_log = DataPoint::builder("commands")
    .field("command", command_name)
    .field("action", action_name)
    .timestamp(Utc::now().timestamp_nanos_opt().unwrap())
    .build()
    .unwrap();

  let _ = influxdb
    .client
    .write(&config.influxdb_bucket, stream::iter(vec![command_log]))
    .await;

  Ok(HttpResponse::NoContent().finish())
}
