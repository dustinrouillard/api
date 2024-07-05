use std::future::{ready, Ready};

use actix_web::{
  dev::{
    forward_ready, Service, ServiceRequest, ServiceResponse, Transform,
  },
  Error,
};
use futures_util::future::LocalBoxFuture;

use crate::connectivity::metrics::ApiMetrics;

pub struct ResponseMeta;

impl<S, B> Transform<S, ServiceRequest> for ResponseMeta
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = Error;
  type InitError = ();
  type Transform = ResponseMetaMiddleware<S>;
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(ResponseMetaMiddleware { service }))
  }
}

pub struct ResponseMetaMiddleware<S> {
  service: S,
}

impl<S, B> Service<ServiceRequest> for ResponseMetaMiddleware<S>
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = Error;
  type Future =
    LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

  forward_ready!(service);

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let start_time = chrono::Utc::now().time();
    let fut = self.service.call(req);

    Box::pin(async move {
      let res = fut.await?;
      let end_time = chrono::Utc::now().time();

      let status_code =
        format!("{:.1$}xx", res.status().as_u16() / 100, 0);
      let response_time = (end_time - start_time).num_milliseconds();

      ApiMetrics::track_request(status_code.into(), response_time as f64);

      Ok(res)
    })
  }
}
