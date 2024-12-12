use std::fmt::Display;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;

use futures::{Future, FutureExt};
use http::{header, Request, Response};
use http_body::Body;
use opentelemetry::trace::{FutureExt as OtelFutureExt, Status, TraceContextExt, Tracer};
use opentelemetry::{global, Context as OtelContext, KeyValue};
use opentelemetry_datadog::new_pipeline;
use opentelemetry_semantic_conventions::attribute::{
    HTTP_CLIENT_IP, HTTP_FLAVOR, HTTP_HOST, HTTP_METHOD, HTTP_SCHEME, HTTP_STATUS_CODE, HTTP_URL,
    HTTP_USER_AGENT,
};
use tower::{Layer, Service};

pub use opentelemetry_datadog::ApiVersion;

/// Initialize the Datadog exporter
pub fn init(service_name: &str, endpoint: &str, version: ApiVersion) {
    let _tracer = new_pipeline()
        .with_service_name(service_name)
        .with_api_version(version)
        .with_agent_endpoint(endpoint)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("failed to initialize tracing pipeline");
}

#[derive(Clone, Debug)]
pub struct DDTraceLayer {
    operation: String,
}

impl DDTraceLayer {
    pub fn new(operation: String) -> DDTraceLayer {
        DDTraceLayer { operation }
    }
}

impl<S> Layer<S> for DDTraceLayer {
    type Service = DDTrace<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DDTrace::new(inner, &self.operation[..])
    }
}

#[derive(Clone, Debug)]
pub struct DDTrace<S> {
    inner: S,
    operation: String,
}

impl<S> DDTrace<S> {
    pub fn new(inner: S, operation: &str) -> Self {
        DDTrace {
            inner,
            operation: operation.to_string(),
        }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for DDTrace<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + 'static,
    S::Error: Display + 'static,
    S::Future: Send,
    ReqBody: 'static,
    ResBody: Body + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let method = req.method().to_string();
        let path = req.uri().path().to_owned();
        let url = req.uri().to_owned().to_string();
        let version = format!("{:?}", req.version());
        let user_agent = req
            .headers()
            .get(header::USER_AGENT)
            .map_or("", |v| v.to_str().unwrap_or(""))
            .to_string();
        let host = req
            .headers()
            .get(header::HOST)
            .map_or("", |v| v.to_str().unwrap_or(""))
            .to_string();
        let scheme = req
            .uri()
            .scheme()
            .map_or_else(|| "http".to_string(), |v| v.to_string());
        let client_ip = parse_x_forwarded_for(req.headers())
            .unwrap_or("")
            .to_string();

        let operation = self.operation.clone();
        let start_time = SystemTime::now();

        let tracer = global::tracer(operation);
        let span = tracer
            .span_builder(path)
            .with_attributes(vec![
                KeyValue::new(HTTP_URL, url),
                KeyValue::new(HTTP_METHOD, method),
                KeyValue::new(HTTP_FLAVOR, version),
                KeyValue::new(HTTP_USER_AGENT, user_agent),
                KeyValue::new(HTTP_HOST, host),
                KeyValue::new(HTTP_SCHEME, scheme),
                KeyValue::new(HTTP_CLIENT_IP, client_ip),
            ])
            .with_start_time(start_time)
            .start(&tracer);

        let cx = OtelContext::current_with_span(span);
        let fut = self
            .inner
            .call(req)
            .with_context(cx.clone())
            .map(move |res| match res {
                Ok(ok_res) => {
                    let span = cx.span();
                    span.set_attribute(KeyValue::new(
                        HTTP_STATUS_CODE,
                        ok_res.status().as_u16() as i64,
                    ));
                    if ok_res.status().is_server_error() {
                        span.set_status(Status::error(
                            ok_res
                                .status()
                                .canonical_reason()
                                .map(|s| s.to_string())
                                .unwrap_or_default(),
                        ));
                    }
                    span.end();
                    Ok(ok_res)
                }
                Err(err_res) => {
                    let span = cx.span();
                    span.set_attribute(KeyValue::new(HTTP_STATUS_CODE, 500));
                    span.set_attribute(KeyValue::new("error.msg", err_res.to_string()));
                    span.set_status(Status::error(err_res.to_string()));
                    span.end();
                    Err(err_res)
                }
            });
        Box::pin(fut)
    }
}

fn parse_x_forwarded_for(headers: &header::HeaderMap) -> Option<&str> {
    let v = headers.get("X-Forwarded-For")?;
    let v = v.to_str().ok()?;
    let mut ips = v.split(',');
    Some(ips.next()?.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_x_forwarded_for() {
        let mut map = header::HeaderMap::new();
        map.insert(
            "X-Forwarded-For",
            "203.0.113.195, 203.0.113.194, 203.0.113.193"
                .parse()
                .unwrap(),
        );

        assert_eq!(parse_x_forwarded_for(&map), Some("203.0.113.195"));
    }
}
