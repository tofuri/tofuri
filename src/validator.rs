use crate::{
    blockchain::Blockchain, cli::ValidatorArgs, constants::SYNC_HISTORY_LENGTH, db, heartbeat,
    http, p2p::MyBehaviour, print, types, util, wallet::Wallet,
};
use ed25519_dalek::Keypair;
use libp2p::{
    futures::{FutureExt, StreamExt},
    Multiaddr, Swarm,
};
use log::error;
use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::net::TcpListener;
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Synchronizer {
    pub new: usize,
    pub bps: usize, // new blocks per second
    pub history: [usize; SYNC_HISTORY_LENGTH],
}
impl Default for Synchronizer {
    fn default() -> Self {
        Self::new()
    }
}
impl Synchronizer {
    pub fn new() -> Synchronizer {
        Synchronizer {
            new: 0,
            bps: 9,
            history: [9; SYNC_HISTORY_LENGTH],
        }
    }
    pub fn heartbeat_handle(&mut self) {
        self.history.rotate_right(1);
        self.history[0] = self.new;
        self.new = 0;
        self.bps = 0;
        for x in self.history {
            self.bps += x;
        }
        self.bps /= SYNC_HISTORY_LENGTH;
    }
}
pub struct Validator {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub blockchain: Blockchain,
    pub keypair: Keypair,
    pub multiaddrs: Vec<Multiaddr>,
    pub synchronizer: Synchronizer,
    pub heartbeats: types::Heartbeats,
    pub lag: [f64; 3],
}
impl Validator {
    pub fn new(
        wallet: Wallet,
        db: DBWithThreadMode<SingleThreaded>,
        known: Vec<Multiaddr>,
    ) -> Result<Validator, Box<dyn Error>> {
        let keypair = wallet.keypair;
        let mut multiaddrs = known;
        multiaddrs.append(&mut Validator::get_multiaddrs(&db)?);
        let blockchain = Blockchain::new(&db)?;
        Ok(Validator {
            db,
            blockchain,
            keypair,
            multiaddrs,
            synchronizer: Synchronizer::new(),
            heartbeats: 0,
            lag: [0.0; 3],
        })
    }
    pub fn put_multiaddr(
        db: &DBWithThreadMode<SingleThreaded>,
        multiaddr: &Multiaddr,
        timestamp: types::Timestamp,
    ) {
        db.put_cf(
            &db::cf_handle_multiaddr(db).unwrap(),
            multiaddr,
            timestamp.to_le_bytes(),
        )
        .unwrap();
    }
    pub fn get_multiaddrs(
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<Vec<Multiaddr>, Box<dyn Error>> {
        let mut multiaddrs = vec![];
        for (i, _) in db.iterator_cf(db::cf_handle_multiaddr(db)?, IteratorMode::Start) {
            multiaddrs.push(String::from_utf8(i.to_vec())?.parse()?);
        }
        Ok(multiaddrs)
    }
    pub fn get_known(args: &ValidatorArgs) -> Result<Vec<Multiaddr>, Box<dyn Error>> {
        let lines = util::read_lines(&args.known)?;
        let mut known = vec![];
        for line in lines {
            match line.parse() {
                Ok(multiaddr) => {
                    known.push(multiaddr);
                }
                Err(err) => error!("{}", err),
            }
        }
        Ok(known)
    }
    pub async fn listen(
        swarm: &mut Swarm<MyBehaviour>,
        listener: TcpListener,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            tokio::select! {
                _ = heartbeat::next().fuse() => heartbeat::handle(swarm)?,
                Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handle(stream, swarm).await {
                    error!("{}", err)
                },
                event = swarm.select_next_some() => print::p2p_event("SwarmEvent", format!("{:?}", event)),
            }
        }
    }
}
