use axum::routing::get;
use axum::routing::post;
use axum::Router;
use clap::Parser;
use pea_api::router;
use pea_core::*;
use std::error::Error;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = pea_api::Args::parse();
    if args.dev {
        if args.api == API {
            args.api = DEV_API.to_string();
        }
        if args.api_internal == API_INTERNAL {
            args.api_internal = DEV_API_INTERNAL.to_string();
        }
    }
    let addr: SocketAddr = args.api.parse().unwrap();
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
        .route("/cargo_pkg_name", get(router::cargo_pkg_name))
        .route("/cargo_pkg_version", get(router::cargo_pkg_version))
        .route("/cargo_pkg_repository", get(router::cargo_pkg_repository))
        .route("/git_hash", get(router::git_hash))
        .layer(cors)
        .with_state(args);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
    Ok(())
}
