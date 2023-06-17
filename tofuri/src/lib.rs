pub mod command;
pub mod interval;
pub mod rpc;
pub mod swarm;
use clap::Parser;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use std::net::IpAddr;
use tofuri_blockchain::Blockchain;
use tofuri_key::Key;
use tofuri_p2p::P2p;
pub const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const SHARE_PEERS_MAX_LEN: usize = 100;
pub struct Node {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Option<Key>,
    pub args: Args,
    pub p2p: P2p,
    pub blockchain: Blockchain,
    pub ticks: usize,
}
impl Node {
    pub fn new(
        db: DBWithThreadMode<SingleThreaded>,
        key: Option<Key>,
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
    /// Store blockchain in a temporary database
    #[clap(long, env = "TEMPDB")]
    pub tempdb: bool,

    /// Generate genesis block
    #[clap(long, env = "MINT")]
    pub mint: bool,

    /// Use testnet instead of mainnet
    #[clap(long, env = "TESTNET")]
    pub testnet: bool,

    /// Trust fork after blocks
    #[clap(long, env = "TRUST", default_value_t = 2)]
    pub trust: usize,

    /// Allow timestamps from the future
    #[clap(long, env = "TIME_DELTA", default_value_t = 1)]
    pub time_delta: u32,

    /// Timeout
    #[clap(long, env = "TIMEOUT", default_value_t = 10000)]
    pub timeout: u64,

    /// TCP socket address to bind to
    #[clap(long, env = "RPC", default_value = "[::]:2021")]
    pub rpc: String,

    /// IpAddr to dial
    #[clap(long, env = "PEER")]
    pub peer: Option<IpAddr>,

    /// Swarm connection limits
    #[clap(long, env = "MAX_ESTABLISHED")]
    pub max_established: Option<u32>,

    /// Secret key
    #[clap(long, env = "SECRET")]
    pub secret: Option<String>,

    /// Disable tracing_subscriber timestamps
    #[clap(long, env = "WITHOUT_TIME")]
    pub without_time: bool,
}
