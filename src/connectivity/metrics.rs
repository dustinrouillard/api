use prometheus::{
  Histogram, HistogramOpts, IntCounterVec, Opts, Registry,
};

use lazy_static::lazy_static;

#[warn(dead_code)]
#[derive(Clone)]
pub struct ApiMetrics {}

lazy_static! {
  pub static ref REGISTRY: Registry = Registry::new();
  pub static ref INCOMING_REQUESTS: IntCounterVec = IntCounterVec::new(
    Opts::new("dstn_api_http_requests", "Incoming Requests"),
    &["status"]
  )
  .expect("metric can be created");
  pub static ref COMMANDS_RUN: IntCounterVec = IntCounterVec::new(
    Opts::new("dstn_api_commands_run", "Commands Run"),
    &["command", "action"]
  )
  .expect("metric can be created");
  pub static ref RESPONSE_TIME_COLLECTOR: Histogram =
    Histogram::with_opts(HistogramOpts::new(
      "dstn_api_response_time",
      "Response Times"
    ))
    .expect("metric can be created");
}

impl ApiMetrics {
  pub async fn new() -> Result<Self, bool> {
    REGISTRY
      .register(Box::new(COMMANDS_RUN.clone()))
      .expect("collector can be registered");

    REGISTRY
      .register(Box::new(INCOMING_REQUESTS.clone()))
      .expect("collector can be registered");

    REGISTRY
      .register(Box::new(RESPONSE_TIME_COLLECTOR.clone()))
      .expect("collector can be registered");

    Ok(Self {})
  }

  pub fn track_request(status_code: String, response_time: f64) {
    RESPONSE_TIME_COLLECTOR.observe(response_time);

    INCOMING_REQUESTS
      .with_label_values(&[&status_code.to_string()])
      .inc()
  }
}
