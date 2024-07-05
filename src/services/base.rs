use actix_web::{get, http::Error, HttpResponse};
use prometheus::Encoder;
use serde_json::json;

pub async fn index() -> Result<HttpResponse, Error> {
  Ok(HttpResponse::NotFound().json(json!({"code": "route_not_found"})))
}

#[get("/health")]
async fn health() -> Result<HttpResponse, Error> {
  Ok(HttpResponse::NoContent().finish())
}

#[get("/metrics")]
async fn get_metrics() -> Result<HttpResponse, Error> {
  let registry = &crate::connectivity::metrics::REGISTRY;

  let encoder = prometheus::TextEncoder::new();

  let mut buffer = Vec::new();
  if let Err(e) = encoder.encode(&registry.gather(), &mut buffer) {
    eprintln!("could not encode custom metrics: {}", e);
  };
  let mut res = match String::from_utf8(buffer.clone()) {
    Ok(v) => v,
    Err(e) => {
      eprintln!("custom metrics could not be from_utf8'd: {}", e);
      String::default()
    }
  };
  buffer.clear();

  let mut buffer = Vec::new();
  if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
    eprintln!("could not encode prometheus metrics: {}", e);
  };
  res.push_str("\n");
  res.push_str(&match String::from_utf8(buffer.clone()) {
    Ok(v) => v,
    Err(e) => {
      eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
      String::default()
    }
  });
  buffer.clear();

  Ok(HttpResponse::Ok().body(res))
}
