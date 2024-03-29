pub mod api;
pub mod interval;
pub mod swarm;
use blockchain::Blockchain;
use clap::Parser;
use key::Key;
use p2p::P2P;
use rocksdb::DB;
use std::net::IpAddr;
pub const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const SHARE_PEERS_MAX_LEN: usize = 100;
pub struct Node {
    pub db: DB,
    pub key: Option<Key>,
    pub args: Args,
    pub p2p: P2P,
    pub blockchain: Blockchain,
    pub ticks: usize,
}
impl Node {
    pub fn new(db: DB, key: Option<Key>, args: Args, p2p: P2P, blockchain: Blockchain) -> Node {
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

    /// IpAddr to dial
    #[clap(long, env = "PEER")]
    pub peer: Option<IpAddr>,

    /// Swarm connection limits
    #[clap(long, env = "MAX_ESTABLISHED")]
    pub max_established: Option<u32>,

    /// Secret key
    #[clap(long, env = "SECRET")]
    pub secret: Option<String>,

    /// API Endpoint
    #[clap(long, env = "API", default_value = "[::]:2021")]
    pub api: String,

    /// Control endpoint
    #[clap(long, env = "CONTROL", default_value = "127.0.0.1:2022")]
    pub control: String,

    /// Disable tracing_subscriber timestamps
    #[clap(long, env = "WITHOUT_TIME")]
    pub without_time: bool,
}
