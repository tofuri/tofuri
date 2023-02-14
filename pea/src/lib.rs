pub mod api_internal;
pub mod interval;
pub mod swarm;
use clap::Parser;
use colored::*;
use log::info;
use log::warn;
use pea_blockchain::blockchain::Blockchain;
use pea_core::*;
use pea_key::Key;
use pea_p2p::P2p;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use serde::Deserialize;
use serde::Serialize;
pub struct Node {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Key,
    pub args: Args,
    pub p2p: P2p,
    pub blockchain: Blockchain,
    pub ticks: usize,
    pub lag: f64,
}
#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
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
    /// Use time api to adjust time difference
    #[clap(long, value_parser, default_value_t = false)]
    pub time_api: bool,
    /// Trust fork after blocks
    #[clap(long, value_parser, default_value = "2")]
    pub trust: usize,
    /// Mesh peers required to ban stakers that failed to show up
    #[clap(long, value_parser, default_value = "10")]
    pub ban_offline: usize,
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
    /// Timeout
    #[clap(long, value_parser, default_value = "300")]
    pub timeout: u64,
}
pub fn args() -> Args {
    let mut args = Args::parse();
    pea_logger::init(args.debug);
    info!(
        "{} = {{ version = \"{}\" }}",
        env!("CARGO_PKG_NAME").yellow(),
        env!("CARGO_PKG_VERSION").magenta()
    );
    info!("{}/tree/{}", env!("CARGO_PKG_REPOSITORY").yellow(), env!("GIT_HASH").magenta());
    if args.dev {
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
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--mint".cyan(), args.mint.to_string().magenta());
    info!("{} {}", "--time-api".cyan(), args.time_api.to_string().magenta());
    info!("{} {}", "--trust".cyan(), args.trust.to_string().magenta());
    info!("{} {}", "--ban-offline".cyan(), args.ban_offline.to_string().magenta());
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
    }
    args
}
