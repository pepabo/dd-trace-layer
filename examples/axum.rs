use axum::{routing::get, Router};
use dd_trace_layer::{init, ApiVersion, DDTraceLayer};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    init(
        "service_name",
        "http://localhost:8126",
        ApiVersion::Version05,
    );

    let app = Router::new()
        .route("/", get(hello_world))
        .layer(DDTraceLayer::new("operation_name".to_string()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}
