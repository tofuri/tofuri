use clap::Parser;
use colored::*;
use log::info;
use pea_logger as logger;
use pea_pay::processor::PaymentProcessor;
use pea_pay_db as db;
use pea_wallet::Wallet;
use std::error::Error;
use tempdir::TempDir;
use tokio::net::TcpListener;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// TCP socket address to bind to
    #[clap(long, value_parser, default_value = ":::9331")]
    pub bind_http_api: String,
    /// API Endpoint
    #[clap(long, value_parser, default_value = "http://localhost:9332")]
    pub http_api: String,
    /// Store blockchain in a temporary database
    #[clap(long, value_parser, default_value_t = false)]
    pub tempdb: bool,
    /// Ticks per second
    #[clap(long, value_parser, default_value = "1")]
    pub tps: f64,
    /// Confirmations needed
    #[clap(long, value_parser, default_value = "10")]
    pub confirmations: usize,
    /// Charge expires after seconds
    #[clap(long, value_parser, default_value = "7200")]
    pub expires: u32,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    logger::init(args.debug);
    info!("{} {}", "Version".cyan(), env!("CARGO_PKG_VERSION").yellow());
    info!("{} {}", "Commit".cyan(), env!("GIT_HASH").yellow());
    info!("{} {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY").yellow());
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--bind-http-api".cyan(), args.bind_http_api.magenta());
    info!("{} {}", "--http-api".cyan(), args.http_api.magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tps".cyan(), args.tps.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    let tempdir = TempDir::new("pea-pay")?;
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./storage/pea-pay",
    };
    let db = db::open(path);
    let wallet = Wallet::import(&args.wallet, &args.passphrase)?;
    let mut payment_processor = PaymentProcessor::new(db, wallet, args.http_api.to_string(), args.confirmations, args.expires);
    payment_processor.load();
    let listener = TcpListener::bind(args.bind_http_api).await?;
    payment_processor.listen(listener, args.tps).await?;
    Ok(())
}
