use super::{
    block::Block,
    blockchain::Blockchain,
    constants::{BLOCKS_PER_SECOND_THRESHOLD, MAX_STAKE, SYNC_BLOCKS, SYNC_HISTORY_LENGTH},
    db, http,
    p2p::MyBehaviour,
    stake::Stake,
    sync::Sync,
    transaction::Transaction,
    util,
    util::print,
    wallet::{address, Wallet},
};
use crate::{constants::BLOCK_TIME_MIN, util::ValidatorArgs};
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
    io::BufRead,
    time::{Duration, SystemTime},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
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
    pub heartbeats: usize,
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
        timestamp: u64,
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
    async fn http_api_listener_accept(
        listener: &tokio::net::TcpListener,
    ) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
        Ok(listener.accept().await?.0)
    }
    async fn http_api_request_handler(
        mut stream: tokio::net::TcpStream,
        swarm: &mut Swarm<MyBehaviour>,
    ) -> Result<(), Box<dyn Error>> {
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await?;
        let first = match buffer.lines().next() {
            Some(first) => match first {
                Ok(first) => first,
                Err(_) => "".to_string(),
            },
            None => "".to_string(),
        };
        print::http_api_request_handler(&first);
        if http::regex::GET.is_match(&first) {
            if http::regex::INDEX.is_match(&first) {
                stream
                    .write_all(http::format_index(swarm.behaviour()).as_bytes())
                    .await?;
            } else if http::regex::JSON.is_match(&first) {
                stream
                    .write_all(http::format_json(swarm.behaviour()).as_bytes())
                    .await?;
            } else if http::regex::BALANCE.is_match(&first) {
                let address = match http::regex::BALANCE.find(&first) {
                    Some(x) => x.as_str().trim().get(9..).unwrap_or(""),
                    None => "",
                };
                let balance = match address::decode(address) {
                    Ok(public_key) => swarm
                        .behaviour()
                        .validator
                        .blockchain
                        .get_balance(&swarm.behaviour().validator.db, &public_key)?,
                    Err(err) => {
                        error!("{}", err);
                        0
                    }
                };
                stream
                    .write_all(http::format_balance(balance).as_bytes())
                    .await?;
            } else if http::regex::BALANCE_STAKED.is_match(&first) {
                let address = match http::regex::BALANCE_STAKED.find(&first) {
                    Some(x) => x.as_str().trim().get(16..).unwrap_or(""),
                    None => "",
                };
                let balance = match address::decode(address) {
                    Ok(public_key) => swarm
                        .behaviour()
                        .validator
                        .blockchain
                        .get_staked_balance(&swarm.behaviour().validator.db, &public_key)?,
                    Err(err) => {
                        error!("{}", err);
                        0
                    }
                };
                stream
                    .write_all(http::format_balance(balance).as_bytes())
                    .await?;
            } else if http::regex::HEIGHT.is_match(&first) {
                stream
                    .write_all(
                        http::format_height(swarm.behaviour().validator.blockchain.latest_height())
                            .as_bytes(),
                    )
                    .await?;
            } else if http::regex::HASH_BY_HEIGHT.is_match(&first) {
                let height = match http::regex::HASH_BY_HEIGHT.find(&first) {
                    Some(x) => match x.as_str().trim().get(6..) {
                        Some(x) => match x.parse::<usize>() {
                            Ok(x) => x,
                            Err(_) => return Ok(()),
                        },
                        None => return Ok(()),
                    },
                    None => return Ok(()),
                };
                let behaviour = swarm.behaviour();
                if height >= behaviour.validator.blockchain.hashes.len() {
                    return Ok(());
                }
                let hash = behaviour
                    .validator
                    .blockchain
                    .hashes
                    .get(height)
                    .ok_or("height index out of range")?;
                stream
                    .write_all(http::format_hash_by_height(&hex::encode(&hash)).as_bytes())
                    .await?;
            } else {
                stream.write_all(http::format_404().as_bytes()).await?;
            };
        } else if http::regex::POST.is_match(&first) {
            if http::regex::TRANSACTION.is_match(&first) {
                let transaction: Transaction = bincode::deserialize(&hex::decode(
                    &buffer
                        .lines()
                        .nth(5)
                        .ok_or("invalid post transaction 1")??
                        .get(0..304)
                        .ok_or("invalid post transaction 2")?,
                )?)?;
                info!("{:?}", transaction);
                let behaviour = swarm.behaviour_mut();
                let status = match behaviour
                    .validator
                    .blockchain
                    .try_add_transaction(&behaviour.validator.db, transaction)
                {
                    Ok(()) => 1,
                    Err(err) => {
                        error!("{}", err);
                        0
                    }
                };
                stream
                    .write_all(http::format_status(status).as_bytes())
                    .await?;
            } else if http::regex::STAKE.is_match(&first) {
                let stake: Stake = bincode::deserialize(&hex::decode(
                    &buffer
                        .lines()
                        .nth(5)
                        .ok_or("invalid post stake 1")??
                        .get(0..242)
                        .ok_or("invalid post stake 2")?,
                )?)?;
                info!("{:?}", stake);
                let behaviour = swarm.behaviour_mut();
                let status = match behaviour
                    .validator
                    .blockchain
                    .try_add_stake(&behaviour.validator.db, stake)
                {
                    Ok(()) => 1,
                    Err(err) => {
                        error!("{}", err);
                        0
                    }
                };
                stream
                    .write_all(http::format_status(status).as_bytes())
                    .await?;
            } else {
                stream.write_all(http::format_404().as_bytes()).await?;
            };
        } else {
            stream.write_all(http::format_400().as_bytes()).await?;
        };
        stream.flush().await?;
        Ok(())
    }
    pub async fn listen(
        swarm: &mut Swarm<MyBehaviour>,
        listener: TcpListener,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            tokio::select! {
                _ = Validator::heartbeat().fuse() => Validator::heartbeat_handle(swarm)?,
                Ok(stream) = Validator::http_api_listener_accept(&listener).fuse() => Validator::http_api_request_handler(stream, swarm).await?,
                event = swarm.select_next_some() => print::p2p_event("SwarmEvent", format!("{:?}", event)),
            }
        }
    }
    pub fn heartbeat_handle_block(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
        if behaviour.validator.synchronizer.bps >= BLOCKS_PER_SECOND_THRESHOLD {
            return Ok(());
        }
        let mut forge = true;
        if !behaviour.validator.blockchain.stakers.queue.is_empty() {
            if &behaviour.validator.blockchain.stakers.queue[0].0
                != behaviour.validator.keypair.public.as_bytes()
                || util::timestamp()
                    < behaviour.validator.blockchain.latest_block.timestamp + BLOCK_TIME_MIN as u64
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
