use axum::extract::State;
use axum::routing::post;
use axum::Router;
use axum::Server;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
pub fn spawn(handle: Handle<EnvFilter, Registry>, addr: &SocketAddr) {
    let builder = Server::bind(addr);
    let make_service = Router::new()
        .route("/", post(handler))
        .layer(TraceLayer::new_for_http())
        .with_state(handle)
        .into_make_service();
    tokio::spawn(async { builder.serve(make_service).await });
}
async fn handler(State(handle): State<Handle<EnvFilter, Registry>>, body: String) {
    handle.reload(body).unwrap();
}
