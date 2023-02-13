use axum::routing::get;
use axum::routing::post;
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
        .route("/balance/:address", get(router::balance))
        .route("/staked/:address", get(router::staked))
        .route("/height", get(router::height))
        .route("/height/:hash", get(router::height_by_hash))
        .route("/block", get(router::block_latest))
        .route("/hash/:height", get(router::hash_by_height))
        .route("/block/:hash", get(router::block_by_hash))
        .route("/transaction/:hash", get(router::transaction_by_hash))
        .route("/stake/:hash", get(router::stake_by_hash))
        .route("/peers", get(router::peers))
        .route("/peer/:a/:b/:c/:d", get(router::peer))
        .route("/transaction", post(router::transaction))
        .route("/stake", post(router::stake))
        .layer(cors);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
    Ok(())
}
