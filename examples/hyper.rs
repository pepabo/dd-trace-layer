use dd_trace_layer::{init, ApiVersion, DDTraceLayer};
use hyper::{server::Server, Body, Error, Request, Response};
use std::net::SocketAddr;
use tower::{make::Shared, ServiceBuilder};

#[tokio::main]
async fn main() {
    init(
        "service_name",
        "http://localhost:8126",
        ApiVersion::Version05,
    );

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
