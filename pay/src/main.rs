use clap::Parser;
use colored::*;
use log::info;
use pea_logger as logger;
use pea_pay::processor::{Options, PaymentProcessor};
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// Store blockchain in a temporary database
    #[clap(long, value_parser, default_value_t = false)]
    pub tempdb: bool,
    /// Use temporary random keypair
    #[clap(long, value_parser, default_value_t = false)]
    pub tempkey: bool,
    /// Confirmations needed
    #[clap(long, value_parser, default_value = "10")]
    pub confirmations: usize,
    /// Charge expires after seconds
    #[clap(long, value_parser, default_value = "7200")]
    pub expires: u32,
    /// Ticks per second
    #[clap(long, value_parser, default_value = "1")]
    pub tps: f64,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
    /// API Endpoint
    #[clap(long, value_parser, default_value = "http://localhost:9332")]
    pub api: String,
    /// TCP socket address to bind to
    #[clap(long, value_parser, default_value = ":::9331")]
    pub bind_api: String,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();
    logger::init(args.debug);
    info!("{} {}", "Crate".cyan(), env!("CARGO_PKG_NAME").yellow());
    info!("{} {}", "Version".cyan(), env!("CARGO_PKG_VERSION").yellow());
    info!("{} {}", "Commit".cyan(), env!("GIT_HASH").yellow());
    info!("{} {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY").yellow());
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--confirmations".cyan(), args.confirmations.to_string().magenta());
    info!("{} {}", "--expires".cyan(), args.expires.to_string().magenta());
    info!("{} {}", "--tps".cyan(), args.tps.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    info!("{} {}", "--api".cyan(), args.api.magenta());
    info!("{} {}", "--bind-api".cyan(), args.bind_api.magenta());
    let mut payment_processor = PaymentProcessor::new(Options {
        tempdb: args.tempdb,
        tempkey: args.tempkey,
        confirmations: args.confirmations,
        expires: args.expires,
        tps: args.tps,
        wallet: &args.wallet,
        passphrase: &args.passphrase,
        api: args.api,
        bind_api: args.bind_api,
    });
    payment_processor.start().await;
}
