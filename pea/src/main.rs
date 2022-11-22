use clap::Parser;
use colored::*;
use log::info;
use pea::node::{Node, Options};
use pea_logger as logger;
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
    /// Trust fork after blocks
    #[clap(long, value_parser, default_value = "128")]
    pub trust: usize,
    /// Pending blocks limit
    #[clap(long, value_parser, default_value = "10")]
    pub pending: usize,
    /// Swarm connection limits
    #[clap(long, value_parser)]
    pub max_established: Option<u32>,
    /// Ticks per second
    #[clap(long, value_parser, default_value = "5")]
    pub tps: f64,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
    /// Multiaddr to dial
    #[clap(short, long, value_parser, default_value = "")]
    pub peer: String,
    /// TCP socket address to bind to
    #[clap(long, value_parser, default_value = ":::9332")]
    pub bind_api: String,
    /// Multiaddr to listen on
    #[clap(short, long, value_parser, default_value = "/ip4/0.0.0.0/tcp/9333")]
    pub host: String,
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
    info!("{} {}", "--trust".cyan(), args.trust.to_string().magenta());
    info!("{} {}", "--pending".cyan(), args.pending.to_string().magenta());
    info!("{} {}", "--max-established".cyan(), format!("{:?}", args.max_established).magenta());
    info!("{} {}", "--tps".cyan(), args.tps.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    info!("{} {}", "--peer".cyan(), args.peer.magenta());
    info!("{} {}", "--bind-api".cyan(), args.bind_api.magenta());
    info!("{} {}", "--host".cyan(), args.host.magenta());
    let mut node = Node::new(Options {
        tempdb: args.tempdb,
        tempkey: args.tempkey,
        trust: args.trust,
        pending: args.pending,
        max_established: args.max_established,
        tps: args.tps,
        wallet: &args.wallet,
        passphrase: &args.passphrase,
        peer: &args.peer,
        bind_api: args.bind_api,
        host: args.host,
    })
    .await;
    node.start().await;
}
