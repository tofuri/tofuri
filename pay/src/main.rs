use axum::routing::get;
use axum::Router;
use clap::Parser;
use colored::*;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tempdir::TempDir;
use tofuri_address::address;
use tofuri_core::*;
use tofuri_key::Key;
use tofuri_pay::router;
use tofuri_pay::Args;
use tofuri_pay::Pay;
use tofuri_pay::CARGO_PKG_NAME;
use tofuri_pay::CARGO_PKG_REPOSITORY;
use tofuri_pay::CARGO_PKG_VERSION;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
#[tokio::main]
async fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let (layer, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(layer)
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let mut args = Args::parse();
    info!(
        "{}",
        tofuri_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY)
    );
    if args.dev {
        if args.tempdb == TEMP_DB {
            args.tempdb = TEMP_DB_DEV;
        }
        if args.tempkey == TEMP_KEY {
            args.tempkey = TEMP_KEY_DEV;
        }
        if args.api == HTTP_API {
            args.api = HTTP_API_DEV.to_string();
        }
        if args.pay_api == PAY_API {
            args.pay_api = PAY_API_DEV.to_string();
        }
    }
    info!("{:#?}", args);
    if args.dev {
        warn!("{}", "DEVELOPMENT MODE IS ACTIVATED!".yellow());
    }
    let addr: SocketAddr = args.pay_api.parse().unwrap();
    let key = match args.tempkey {
        true => Key::generate(),
        false => {
            tofuri_wallet::load(&args.wallet, &args.passphrase)
                .unwrap()
                .3
        }
    };
    info!(address = address::encode(&key.address_bytes()));
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
                        info!("{:?}", vec);
                    }
                }
                Err(err) => error!("{:?}", err),
            }
        }
    });
    tofuri_util::io_reload_filter(reload_handle);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
