use crate::{
    block::Block,
    blockchain::Blockchain,
    cli::ValidatorArgs,
    constants::{
        BLOCKS_PER_SECOND_THRESHOLD, BLOCK_TIME_MIN, MAX_STAKE, SYNC_BLOCKS, SYNC_HISTORY_LENGTH,
    },
    db, http,
    p2p::MyBehaviour,
    print,
    stake::Stake,
    sync::Sync,
    transaction::Transaction,
    types, util,
    wallet::Wallet,
};
use colored::*;
use ed25519_dalek::Keypair;
use libp2p::{
    futures::{FutureExt, StreamExt},
    gossipsub::{GossipsubMessage, IdentTopic},
    Multiaddr, Swarm,
};
use log::{debug, error, info};
use rocksdb::{DBWithThreadMode, IteratorMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    time::{Duration, SystemTime},
};
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
    async fn heartbeat() {
        let mut nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let secs = nanos / 1_000_000_000;
        nanos -= secs * 1_000_000_000;
        nanos = 1_000_000_000 - nanos;
        tokio::time::sleep(Duration::from_nanos(nanos as u64)).await
    }
    fn heartbeat_handle(swarm: &mut Swarm<MyBehaviour>) -> Result<(), Box<dyn Error>> {
        let behaviour = swarm.behaviour_mut();
        behaviour.validator.heartbeats += 1;
        behaviour.validator.synchronizer.heartbeat_handle();
        Validator::heartbeat_handle_block(behaviour)?;
        Validator::heartbeat_handle_sync(behaviour)?;
        let millis = Validator::heartbeat_lag();
        print::heartbeat_lag(behaviour.validator.heartbeats, millis);
        behaviour.validator.lag.rotate_right(1);
        behaviour.validator.lag[0] = millis;
        Ok(())
    }
    fn heartbeat_lag() -> f64 {
        let mut micros = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let secs = micros / 1_000_000;
        micros -= secs * 1_000_000;
        micros as f64 / 1_000_f64
    }
    async fn http_listener_accept(
        listener: &tokio::net::TcpListener,
    ) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
        Ok(listener.accept().await?.0)
    }
    pub async fn listen(
        swarm: &mut Swarm<MyBehaviour>,
        listener: TcpListener,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            tokio::select! {
                _ = Validator::heartbeat().fuse() => Validator::heartbeat_handle(swarm)?,
                Ok(stream) = Validator::http_listener_accept(&listener).fuse() => if let Err(err) = http::handle(stream, swarm).await {
                    error!("{}", err)
                },
                event = swarm.select_next_some() => print::p2p_event("SwarmEvent", format!("{:?}", event)),
            }
        }
    }
    pub fn heartbeat_handle_block(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
        if behaviour.validator.synchronizer.bps >= BLOCKS_PER_SECOND_THRESHOLD {
            return Ok(());
        }
        let mut forge = true;
        if !behaviour.validator.blockchain.stakers.is_empty() {
            if &behaviour.validator.blockchain.stakers[0].0
                != behaviour.validator.keypair.public.as_bytes()
                || util::timestamp()
                    < behaviour.validator.blockchain.latest_block.timestamp
                        + BLOCK_TIME_MIN as types::Timestamp
            {
                forge = false;
            }
        } else {
            // cold start
            let mut stake = Stake::new(true, MAX_STAKE, 0);
            stake.sign(&behaviour.validator.keypair);
            behaviour.validator.blockchain.pending_stakes.push(stake);
        }
        if forge {
            // forge new block
            match behaviour
                .validator
                .blockchain
                .forge_block(&behaviour.validator.db, &behaviour.validator.keypair)
            {
                Ok(block) => {
                    if behaviour.gossipsub.all_peers().count() > 0 {
                        behaviour
                            .gossipsub
                            .publish(IdentTopic::new("block"), bincode::serialize(&block)?)?;
                    }
                }
                Err(err) => error!("{}", err),
            };
        }
        // accept forged blocks
        if let Err(err) = behaviour
            .validator
            .blockchain
            .accept_block(&behaviour.validator.db, forge)
        {
            debug!("{}", err)
        }
        Ok(())
    }
    pub fn heartbeat_handle_sync(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
        if behaviour.validator.synchronizer.bps < BLOCKS_PER_SECOND_THRESHOLD {
            return Ok(());
        }
        info!(
            "{}: {} @ {}bps",
            "Synchronize".cyan(),
            behaviour
                .validator
                .blockchain
                .latest_height()
                .to_string()
                .yellow(),
            behaviour.validator.synchronizer.bps.to_string().yellow()
        );
        if behaviour.gossipsub.all_peers().count() > 0 {
            behaviour.gossipsub.publish(
                IdentTopic::new("sync"),
                bincode::serialize(&Sync::new(
                    behaviour.validator.blockchain.latest_height() + 1,
                ))?,
            )?;
        }
        Ok(())
    }
    pub fn gossipsub_message_handler(
        behaviour: &mut MyBehaviour,
        message: GossipsubMessage,
    ) -> Result<(), Box<dyn Error>> {
        match message.topic.as_str() {
            "block" => {
                let block: Block = bincode::deserialize(&message.data)?;
                let previous_hash = block.previous_hash;
                behaviour
                    .validator
                    .blockchain
                    .try_add_block(&behaviour.validator.db, block)?;
                if behaviour.validator.synchronizer.bps >= BLOCKS_PER_SECOND_THRESHOLD {
                    // accept block early for faster synchronization
                    behaviour
                        .validator
                        .blockchain
                        .accept_block(&behaviour.validator.db, false)?
                }
                if behaviour.validator.blockchain.latest_block.previous_hash == previous_hash {
                    behaviour.validator.synchronizer.new += 1;
                }
            }
            "stake" => {
                let stake: Stake = bincode::deserialize(&message.data)?;
                behaviour
                    .validator
                    .blockchain
                    .try_add_stake(&behaviour.validator.db, stake)?;
            }
            "transaction" => {
                let transaction: Transaction = bincode::deserialize(&message.data)?;
                behaviour
                    .validator
                    .blockchain
                    .try_add_transaction(&behaviour.validator.db, transaction)?;
            }
            "ip" => {}
            "sync" => {
                let sync: Sync = bincode::deserialize(&message.data)?;
                for i in sync.height..=sync.height + SYNC_BLOCKS {
                    if i > behaviour.validator.blockchain.latest_height() {
                        return Ok(());
                    }
                    let hash = behaviour.validator.blockchain.hashes.get(i).unwrap();
                    if behaviour.gossipsub.all_peers().count() > 0 {
                        behaviour.gossipsub.publish(
                            IdentTopic::new("block"),
                            bincode::serialize(&Block::get(&behaviour.validator.db, hash)?)?,
                        )?;
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
}
