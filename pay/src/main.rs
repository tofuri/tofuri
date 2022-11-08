use clap::Parser;
use log::info;
use pea_logger as logger;
use pea_pay::processor::PaymentProcessor;
use pea_wallet::Wallet;
use std::error::Error;
use tokio::net::TcpListener;
const HTTP_API: &str = "http://localhost:9332";
const CONFIRMATIONS: usize = 10;
const EXPIRES_AFTER_SECS: u32 = 60;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// API Endpoint
    #[clap(long, value_parser, default_value = ":::9331")]
    pub http_api: String,
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
    let wallet = Wallet::import(&args.wallet, &args.passphrase)?;
    let mut payment_processor = PaymentProcessor::new(wallet, HTTP_API.to_string(), CONFIRMATIONS, EXPIRES_AFTER_SECS);
    let payment = payment_processor.charge(10000000000);
    info!("{:?}", payment);
    let listener = TcpListener::bind(args.http_api).await?;
    payment_processor.listen(listener).await?;
    Ok(())
}
