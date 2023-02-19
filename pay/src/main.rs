use axum::routing::get;
use axum::Router;
use clap::Parser;
use colored::*;
use log::error;
use log::info;
use pea_address::address;
use pea_key::Key;
use pea_pay::pay::Options;
use pea_pay::pay::Pay;
use pea_pay::router;
use pea_pay::Args;
use pea_wallet::wallet;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tempdir::TempDir;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    pea_logger::init(args.debug);
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--confirmations".cyan(), args.confirmations.to_string().magenta());
    info!("{} {}", "--expires".cyan(), args.expires.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    info!("{} {}", "--api".cyan(), args.api.magenta());
    info!("{} {}", "--pay_api".cyan(), args.pay_api.magenta());
    info!("{} {}", "--bind-api".cyan(), args.bind_api.magenta());
    let key = match args.tempkey {
        true => Key::generate(),
        false => wallet::load(&args.wallet, &args.passphrase).unwrap().3,
    };
    info!("Address {}", address::encode(&key.address_bytes()).green());
    let tempdir = TempDir::new("peacash-pay-db").unwrap();
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./peacash-pay-db",
    };
    let db = pea_pay_db::open(path);
    let pay = Arc::new(Mutex::new(Pay::new(
        key,
        db,
        Options {
            tempdb: args.tempdb,
            tempkey: args.tempkey,
            confirmations: args.confirmations,
            expires: args.expires,
            wallet: &args.wallet,
            passphrase: &args.passphrase,
            api: args.api,
            bind_api: args.bind_api,
        },
    )));
    let addr: SocketAddr = args.pay_api.parse().unwrap();
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
