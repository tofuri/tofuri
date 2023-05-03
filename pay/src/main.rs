use axum::routing::get;
use axum::Router;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tempdir::TempDir;
use tofuri_address::address;
use tofuri_pay::router;
use tofuri_pay::Args;
use tofuri_pay::Pay;
use tofuri_pay::CARGO_PKG_NAME;
use tofuri_pay::CARGO_PKG_REPOSITORY;
use tofuri_pay::CARGO_PKG_VERSION;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::debug;
use tracing::error;
use tracing::info;
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
    let addr: SocketAddr = args.pay_api.parse().unwrap();
    let key = tofuri_pay::key(args.tempkey, &args.secret);
    let address = address::encode(&key.address_bytes());
    info!(address);
    let tempdir = TempDir::new("tofuri-pay-db").unwrap();
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./tofuri-pay-db",
    };
    let db = tofuri_pay_db::open(path);
    let pay = Arc::new(Mutex::new(Pay::new(db, key, args)));
    let cors = CorsLayer::permissive();
    let app = Router::new()
        .route("/", get(router::root))
        .route("/charges", get(router::charges))
        .route("/charge/:hash", get(router::charge))
        .route("/charge/new/:amount", get(router::charge_new))
        .layer(cors)
        .with_state(pay.clone());
    tokio::spawn(async move {
        pay.lock().await.load().unwrap();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            match pay.lock().await.check().await {
                Ok(vec) => {
                    if !vec.is_empty() {
                        info!(?vec);
                    }
                }
                Err(e) => error!(?e),
            }
        }
    });
    tofuri_util::io_reload_filter(reload_handle);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
