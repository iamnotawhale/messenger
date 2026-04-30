use axum::{routing::get, Json, Router};
use messenger_protocol::{TransportKind, PROTOCOL_VERSION};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    protocol_version: u16,
    transports: [TransportKind; 2],
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/signaling", get(signaling_placeholder))
        .route("/v1/relay", get(relay_placeholder));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap_or_else(|error| panic!("failed to bind server: {error}"));

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|error| panic!("server failed: {error}"));
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        protocol_version: PROTOCOL_VERSION,
        transports: [TransportKind::Relay, TransportKind::WebRtc],
    })
}

async fn signaling_placeholder() -> &'static str {
    "signaling websocket endpoint placeholder"
}

async fn relay_placeholder() -> &'static str {
    "encrypted relay queue endpoint placeholder"
}
