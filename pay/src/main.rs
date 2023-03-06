use axum::routing::get;
use axum::Router;
use clap::Parser;
use colored::*;
use log::error;
use log::info;
use log::warn;
use std::error::Error;
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
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let mut args = Args::parse();
    info!("{}", tofuri_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY));
    if args.dev {
        if args.tempdb == TEMP_DB {
            args.tempdb = DEV_TEMP_DB;
        }
        if args.tempkey == TEMP_KEY {
            args.tempkey = DEV_TEMP_KEY;
        }
        if args.api == HTTP_API {
            args.api = DEV_HTTP_API.to_string();
        }
        if args.pay_api == PAY_API {
            args.pay_api = DEV_PAY_API.to_string();
        }
    }
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--confirmations".cyan(), args.confirmations.to_string().magenta());
    info!("{} {}", "--expires".cyan(), args.expires.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    info!("{} {}", "--api".cyan(), args.api.magenta());
    info!("{} {}", "--pay_api".cyan(), args.pay_api.magenta());
    info!("{} {}", "--dev".cyan(), args.dev.to_string().magenta());
    if args.dev {
        warn!("{}", "DEVELOPMENT MODE IS ACTIVATED!".yellow());
    }
    let addr: SocketAddr = args.pay_api.parse().unwrap();
    let key = match args.tempkey {
        true => Key::generate(),
        false => tofuri_wallet::load(&args.wallet, &args.passphrase).unwrap().3,
    };
    info!("Address {}", address::encode(&key.address_bytes()).green());
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
        pay.lock().await.load();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            match pay.lock().await.check().await {
                Ok(vec) => {
                    if !vec.is_empty() {
                        info!("{:?}", vec);
                    }
                }
                Err(err) => error!("{}", err),
            }
        }
    });
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
    Ok(())
}
