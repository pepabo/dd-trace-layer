# dd-trace-layer
`dd-trace-layer` is a middleware for sending Datadog's trace. It's based on [Tower](https://github.com/tower-rs/tower) and [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust).

## Usage
This can be used in [`hyper`](https://github.com/hyperium/hyper) or [`axum`](https://github.com/tokio-rs/axum), etc. See [examples](./examples).

```rust
use dd_trace_layer::{init, ApiVersion, DDTraceLayer};
use hyper::{server::Server, Body, Error, Request, Response};
use std::net::SocketAddr;
use tower::{make::Shared, ServiceBuilder};

#[tokio::main]
async fn main() {
    init("service_name", "http://localhost:8126", ApiVersion::Version05);

    let service = ServiceBuilder::new()
        .layer(DDTraceLayer::new("operation_name".to_string()))
        .service_fn(hello_world);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    Server::bind(&addr)
        .serve(Shared::new(service))
        .await
        .expect("server error");
}

async fn hello_world(_: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(Response::new(Body::from("Hello, World!")))
}
```

## Contributing
1. Fork
2. Create a feature branch
3. Commit your changes
4. Rebase your local changes against the main branch
5. Run test suite with the `cargo test` command and confirm that it passes
6. Run `cargo fmt` and pass `cargo clippy`
7. Create new Pull Request