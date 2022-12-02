use clap::Parser;
use colored::*;
use log::{info, warn};
use pea::node::{Node, Options};
use pea_logger as logger;
const TEMP_DB: bool = false;
const TEMP_KEY: bool = false;
const BIND_API: &str = ":::9332";
const HOST: &str = "/ip4/0.0.0.0/tcp/9333";
const DEV_TEMP_DB: bool = true;
const DEV_TEMP_KEY: bool = true;
const DEV_BIND_API: &str = ":::9334";
const DEV_HOST: &str = "/ip4/0.0.0.0/tcp/9335";
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// Store blockchain in a temporary database
    #[clap(long, value_parser, default_value_t = TEMP_DB)]
    pub tempdb: bool,
    /// Use temporary random keypair
    #[clap(long, value_parser, default_value_t = TEMP_KEY)]
    pub tempkey: bool,
    /// Generate genesis block
    #[clap(long, value_parser, default_value_t = false)]
    pub mint: bool,
    /// Trust fork after blocks
    #[clap(long, value_parser, default_value = "16")]
    pub trust: usize,
    /// Pending blocks limit
    #[clap(long, value_parser, default_value = "256")]
    pub pending: usize,
    /// Mesh peers required to ban stakers that failed to show up
    #[clap(long, value_parser, default_value = "10")]
    pub ban_offline: usize,
    /// Time synchronization requests to measure average delay
    #[clap(long, value_parser, default_value = "2")]
    pub time_sync_requests: usize,
    /// Max time delta allowed
    #[clap(long, value_parser, default_value = "1")]
    pub time_delta: u32, // ping delay & perception of time
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
    #[clap(long, value_parser, default_value = BIND_API)]
    pub bind_api: String,
    /// Multiaddr to listen on
    #[clap(short, long, value_parser, default_value = HOST)]
    pub host: String,
    /// Development mode
    #[clap(long, value_parser, default_value_t = false)]
    pub dev: bool,
}
#[tokio::main]
async fn main() {
    println!(
        "{} = {{ version = \"{}\" }}",
        env!("CARGO_PKG_NAME").yellow(),
        env!("CARGO_PKG_VERSION").magenta()
    );
    println!("{}/tree/{}", env!("CARGO_PKG_REPOSITORY").yellow(), env!("GIT_HASH").magenta());
    let mut args = Args::parse();
    logger::init(args.debug);
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--mint".cyan(), args.mint.to_string().magenta());
    info!("{} {}", "--trust".cyan(), args.trust.to_string().magenta());
    info!("{} {}", "--pending".cyan(), args.pending.to_string().magenta());
    info!("{} {}", "--ban-offline".cyan(), args.ban_offline.to_string().magenta());
    info!("{} {}", "--time-sync-requests".cyan(), args.time_sync_requests.to_string().magenta());
    info!("{} {}", "--time-delta".cyan(), args.time_delta.to_string().magenta());
    info!("{} {}", "--max-established".cyan(), format!("{:?}", args.max_established).magenta());
    info!("{} {}", "--tps".cyan(), args.tps.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    info!("{} {}", "--peer".cyan(), args.peer.magenta());
    info!("{} {}", "--bind-api".cyan(), args.bind_api.magenta());
    info!("{} {}", "--host".cyan(), args.host.magenta());
    info!("{} {}", "--dev".cyan(), args.dev.to_string().magenta());
    if args.dev {
        warn!("{}", "DEVELOPMENT MODE IS ACTIVATED!".yellow());
        if args.tempdb == TEMP_DB {
            args.tempdb = DEV_TEMP_DB;
        }
        if args.tempkey == TEMP_KEY {
            args.tempkey = DEV_TEMP_KEY;
        }
        if args.bind_api == BIND_API {
            args.bind_api = DEV_BIND_API.to_string();
        }
        if args.host == HOST {
            args.host = DEV_HOST.to_string();
        }
    }
    let mut node = Node::new(Options {
        tempdb: args.tempdb,
        tempkey: args.tempkey,
        mint: args.mint,
        trust: args.trust,
        pending: args.pending,
        ban_offline: args.ban_offline,
        time_sync_requests: args.time_sync_requests,
        time_delta: args.time_delta,
        max_established: args.max_established,
        tps: args.tps,
        wallet: &args.wallet,
        passphrase: &args.passphrase,
        peer: &args.peer,
        bind_api: args.bind_api,
        host: args.host,
        dev: args.dev,
    })
    .await;
    node.start().await;
}
