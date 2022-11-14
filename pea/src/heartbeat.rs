use crate::p2p::MyBehaviour;
use colored::*;
use libp2p::{gossipsub::IdentTopic, Swarm};
use log::{debug, error};
use pea_core::{
    constants::{BLOCK_TIME_MIN, MIN_STAKE, SYNC_BLOCKS_PER_TICK},
    util,
};
use pea_stake::Stake;
use std::time::{Duration, SystemTime};
pub async fn next(tps: f64) {
    tokio::time::sleep(Duration::from_nanos(nanos(tps))).await
}
pub fn handler(swarm: &mut Swarm<MyBehaviour>) {
    let behaviour = swarm.behaviour_mut();
    behaviour.heartbeats += 1;
    sync(behaviour);
    behaviour.message_data_hashes.clear();
    behaviour.blockchain.sync.handler();
    forge(behaviour);
    behaviour.blockchain.pending_blocks_accept();
    lag(behaviour);
}
fn forge(behaviour: &mut MyBehaviour) {
    let states = &behaviour.blockchain.states;
    if behaviour.blockchain.sync.syncing {
        return;
    }
    let timestamp = util::timestamp();
    if let Some(public_key) = states.dynamic.staker(timestamp, states.dynamic.latest_block.timestamp) {
        if public_key != &behaviour.blockchain.key.public_key_bytes() || timestamp < states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
            return;
        }
    } else {
        let mut stake = Stake::new(true, MIN_STAKE, 0);
        stake.sign(&behaviour.blockchain.key);
        behaviour.blockchain.set_cold_start_stake(stake);
    }
    let block = behaviour.blockchain.forge_block().unwrap();
    if behaviour.gossipsub.all_peers().count() == 0 {
        return;
    }
    let data = bincode::serialize(&block).unwrap();
    if behaviour.filter(&data, true) {
        return;
    }
    if let Err(err) = behaviour.gossipsub.publish(IdentTopic::new("block"), data) {
        error!("{}", err);
    }
}
fn sync(behaviour: &mut MyBehaviour) {
    if behaviour.blockchain.states.dynamic.hashes.is_empty() {
        return;
    }
    if behaviour.gossipsub.all_peers().count() == 0 {
        behaviour.blockchain.sync.index_0 = 0;
        return;
    }
    for _ in 0..SYNC_BLOCKS_PER_TICK {
        for block in behaviour.blockchain.sync_blocks() {
            let data = bincode::serialize(&block).unwrap();
            if let Err(err) = behaviour.gossipsub.publish(IdentTopic::new("block"), data) {
                error!("{}", err);
            }
        }
    }
}
fn nanos(tps: f64) -> u64 {
    let f = 1_f64 / tps;
    let u = (f * 1_000_000_000_f64) as u128;
    let mut nanos = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let secs = nanos / u;
    nanos -= secs * u;
    (u - nanos) as u64
}
fn lag(behaviour: &mut MyBehaviour) {
    let f = 1_f64 / behaviour.tps;
    let u = (f * 1_000_000_000_f64) as u64;
    let nanos = u - nanos(behaviour.tps);
    behaviour.lag = (nanos / 1_000) as f64 / 1_000_f64;
    debug!("{} {} {}", "Heartbeat".cyan(), behaviour.heartbeats, format!("{:?}", Duration::from_nanos(nanos)).yellow());
}
