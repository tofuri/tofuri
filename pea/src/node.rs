use crate::http;
use crate::interval;
use crate::swarm;
use clap::Parser;
use colored::*;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
use log::error;
use log::info;
use pea_address::address;
use pea_blockchain::blockchain::Blockchain;
use pea_db as db;
use pea_key::Key;
use pea_p2p::multiaddr;
use pea_p2p::P2p;
use pea_util;
use pea_wallet::wallet;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::time::Duration;
use tempdir::TempDir;
use tokio::net::TcpListener;
pub const TEMP_DB: bool = false;
pub const TEMP_KEY: bool = false;
pub const BIND_API: &str = ":::9332";
pub const HOST: &str = "/ip4/0.0.0.0/tcp/9333";
pub const DEV_TEMP_DB: bool = true;
pub const DEV_TEMP_KEY: bool = true;
pub const DEV_BIND_API: &str = ":::9334";
pub const DEV_HOST: &str = "/ip4/0.0.0.0/tcp/9335";
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
pub struct Node {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Key,
    pub args: Args,
    pub p2p: P2p,
    pub blockchain: Blockchain,
    pub heartbeats: usize,
    pub lag: f64,
}
impl Node {
    pub async fn new(args: Args) -> Node {
        let key = match args.tempkey {
            true => Key::generate(),
            false => wallet::load(&args.wallet, &args.passphrase).unwrap().3,
        };
        info!("Address {}", address::encode(&key.address_bytes()).green());
        let tempdir = TempDir::new("peacash-db").unwrap();
        let path: &str = match args.tempdb {
            true => tempdir.path().to_str().unwrap(),
            false => "./peacash-db",
        };
        let db = db::open(path);
        let mut known = HashSet::new();
        if let Some(multiaddr) = multiaddr::ip_port(&args.peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
        let peers = db::peer::get_all(&db);
        for peer in peers {
            if let Some(multiaddr) = multiaddr::ip_port(&peer.parse::<Multiaddr>().unwrap()) {
                known.insert(multiaddr);
            }
        }
        let p2p = P2p::new(args.max_established, args.timeout, known, args.ban_offline).await.unwrap();
        let blockchain = Blockchain::new(args.trust, args.time_delta);
        Node {
            key,
            p2p,
            blockchain,
            db,
            heartbeats: 0,
            lag: 0.0,
            args,
        }
    }
    pub async fn run(&mut self) {
        self.blockchain.load(&self.db);
        info!(
            "Blockchain height is {}",
            if let Some(main) = self.blockchain.tree.main() {
                main.1.to_string().yellow()
            } else {
                "0".red()
            }
        );
        info!("Latest block seen {}", self.blockchain.last_seen().yellow());
        let multiaddr: Multiaddr = self.args.host.parse().unwrap();
        self.p2p.swarm.listen_on(multiaddr.clone()).unwrap();
        info!("Swarm is listening on {}", multiaddr.to_string().magenta());
        let listener = TcpListener::bind(&self.args.bind_api).await.unwrap();
        info!(
            "API is listening on {}{}",
            "http://".cyan(),
            listener.local_addr().unwrap().to_string().magenta()
        );
        let mut interval = tokio::time::interval(Duration::from_micros(pea_util::micros_per_tick(self.args.tps)));
        loop {
            tokio::select! {
                biased;
                instant = interval.tick() => interval::tick(self, instant),
                event = self.p2p.swarm.select_next_some() => swarm::event(self, event),
                res = listener.accept() => match res {
                    Ok((stream, socket_addr)) => {
                        match http::handler(stream, self).await {
                            Ok((bytes, first)) => info!("{} {} {} {}", "API".cyan(), socket_addr.to_string().magenta(), bytes.to_string().yellow(), first),
                            Err(err) => error!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), err)
                        }
                    }
                    Err(err) => error!("{} {}", "API".cyan(), err)
                }
            }
        }
    }
    pub fn uptime(&self) -> String {
        let seconds = (self.heartbeats as f64 / self.args.tps) as u32;
        pea_util::duration_to_string(seconds, "0")
    }
}
