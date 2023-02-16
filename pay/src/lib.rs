pub mod pay;
pub mod router;
use clap::Parser;
use pea_core::*;
#[derive(Parser, Debug, Clone)]
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
    #[clap(long, value_parser, default_value = "20")]
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
    #[clap(long, value_parser, default_value = HTTP_API)]
    pub api: String,
    /// Pay API Endpoint
    #[clap(long, value_parser, default_value = PAY_API)]
    pub pay_api: String,
    /// TCP socket address to bind to
    #[clap(long, value_parser, default_value = ":::9331")]
    pub bind_api: String,
}
