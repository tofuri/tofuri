use crate::http;
use crate::interval;
use crate::swarm;
use crate::Args;
use colored::*;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
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
use std::collections::HashSet;
use std::time::Duration;
use tempdir::TempDir;
use tokio::net::TcpListener;
pub struct Node {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub key: Key,
    pub args: Args,
    pub p2p: P2p,
    pub blockchain: Blockchain,
    pub ticks: usize,
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
            ticks: 0,
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
                res = listener.accept() => http::accept(self, res).await,
            }
        }
    }
    pub fn uptime(&self) -> String {
        let seconds = (self.ticks as f64 / self.args.tps) as u32;
        pea_util::duration_to_string(seconds, "0")
    }
}
