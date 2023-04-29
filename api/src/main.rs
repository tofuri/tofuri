use axum::routing::get;
use axum::routing::post;
use axum::Router;
use clap::Parser;
use std::net::SocketAddr;
use tofuri_api::router;
use tofuri_api::Args;
use tofuri_api::CARGO_PKG_NAME;
use tofuri_api::CARGO_PKG_REPOSITORY;
use tofuri_api::CARGO_PKG_VERSION;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::debug;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
#[tokio::main]
async fn main() {
    println!(
        "{}",
        tofuri_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY)
    );
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let (layer, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(layer)
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let args = Args::parse();
    debug!("{:?}", args);
    let addr: SocketAddr = args.api.parse().unwrap();
    let cors = CorsLayer::permissive();
    let trace = TraceLayer::new_for_http();
    let app = Router::new()
        .route("/", get(router::root))
        .route("/balance/:address", get(router::balance))
        .route(
            "/balance_pending_min/:address",
            get(router::balance_pending_min),
        )
        .route(
            "/balance_pending_max/:address",
            get(router::balance_pending_max),
        )
        .route("/staked/:address", get(router::staked))
        .route(
            "/staked_pending_min/:address",
            get(router::staked_pending_min),
        )
        .route(
            "/staked_pending_max/:address",
            get(router::staked_pending_max),
        )
        .route("/height", get(router::height))
        .route("/height/:hash", get(router::height_by_hash))
        .route("/block", get(router::block_latest))
        .route("/hash/:height", get(router::hash_by_height))
        .route("/block/:hash", get(router::block_by_hash))
        .route("/transaction/:hash", get(router::transaction_by_hash))
        .route("/stake/:hash", get(router::stake_by_hash))
        .route("/peers", get(router::peers))
        .route("/peer/:ip_addr", get(router::peer))
        .route("/transaction", post(router::transaction))
        .route("/stake", post(router::stake))
        .route("/cargo_pkg_name", get(router::cargo_pkg_name))
        .route("/cargo_pkg_version", get(router::cargo_pkg_version))
        .route("/cargo_pkg_repository", get(router::cargo_pkg_repository))
        .route("/git_hash", get(router::git_hash))
        .route("/address", get(router::address))
        .route("/ticks", get(router::ticks))
        .route("/time", get(router::time))
        .route("/tree_size", get(router::tree_size))
        .route("/sync", get(router::sync))
        .route("/random_queue", get(router::random_queue))
        .route("/unstable_hashes", get(router::unstable_hashes))
        .route(
            "/unstable_latest_hashes",
            get(router::unstable_latest_hashes),
        )
        .route("/unstable_stakers", get(router::unstable_stakers))
        .route("/stable_hashes", get(router::stable_hashes))
        .route("/stable_latest_hashes", get(router::stable_latest_hashes))
        .route("/stable_stakers", get(router::stable_stakers))
        .route("/sync_remaining", get(router::sync_remaining))
        .layer(trace)
        .layer(cors)
        .with_state(args);
    tofuri_util::io_reload_filter(reload_handle);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
