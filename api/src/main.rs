use axum::routing::get;
use axum::Router;
use pea_api::router;
use std::error::Error;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cors = CorsLayer::permissive();
    let app = Router::new()
        .route("/", get(router::root))
        .route("/height", get(router::height))
        .route("/block", get(router::block_latest))
        .layer(cors);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
    Ok(())
}
