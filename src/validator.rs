use crate::{
    blockchain::Blockchain, cli::ValidatorArgs, db, heartbeat, http, p2p::MyBehaviour, print,
    synchronizer::Synchronizer, types, util, wallet::Wallet,
};
use libp2p::{
    futures::{FutureExt, StreamExt},
    Multiaddr, Swarm,
};
use log::error;
use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
use std::error::Error;
use tokio::net::TcpListener;
pub struct Validator {
    pub db: DBWithThreadMode<SingleThreaded>,
    pub blockchain: Blockchain,
    pub keypair: types::Keypair,
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
        let mut blockchain = Blockchain::new();
        blockchain.reload(&db);
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
        db.put_cf(db::peers(db), multiaddr, timestamp.to_le_bytes())
            .unwrap();
    }
    pub fn get_multiaddrs(
        db: &DBWithThreadMode<SingleThreaded>,
    ) -> Result<Vec<Multiaddr>, Box<dyn Error>> {
        let mut multiaddrs = vec![];
        for i in db.iterator_cf(db::peers(db), IteratorMode::Start) {
            multiaddrs.push(String::from_utf8(i?.0.to_vec())?.parse()?);
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
                _ = heartbeat::next().fuse() => if let Err(err) = heartbeat::handle(swarm) {
                    error!("{}", err);
                },
                Ok(stream) = http::next(&listener).fuse() => if let Err(err) = http::handle(stream, swarm).await {
                    error!("{}", err);
                },
                event = swarm.select_next_some() => print::p2p_event("SwarmEvent", format!("{:?}", event)),
            }
        }
    }
}
