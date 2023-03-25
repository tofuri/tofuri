pub mod interval;
pub mod rpc;
pub mod swarm;
use clap::Parser;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use tofuri_blockchain::Blockchain;
use tofuri_core::*;
use tofuri_key::Key;
use tofuri_p2p::P2p;
pub const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub struct Node {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Key,
    pub args: Args,
    pub p2p: P2p,
    pub blockchain: Blockchain,
    pub ticks: usize,
}
impl Node {
    pub fn new(
        db: DBWithThreadMode<SingleThreaded>,
        key: Key,
        args: Args,
        p2p: P2p,
        blockchain: Blockchain,
    ) -> Node {
        Node {
            db,
            key,
            args,
            p2p,
            blockchain,
            ticks: 0,
        }
    }
}
#[derive(Parser, Debug, Clone)]
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
    #[clap(long, value_parser, default_value = "2")]
    pub trust: usize,

    /// Allow timestamps from the future
    #[clap(long, value_parser, default_value = "1")]
    pub time_delta: u32,

    /// Swarm connection limits
    #[clap(long, value_parser)]
    pub max_established: Option<u32>,

    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,

    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,

    /// IpAddr to dial
    #[clap(short, long, value_parser, default_value = "")]
    pub peer: String,

    /// TCP socket address to bind to
    #[clap(long, value_parser, default_value = RPC)]
    pub rpc: String,

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
