use crate::p2p::MyBehaviour;
use colored::*;
use libp2p::{gossipsub::IdentTopic, Swarm};
use log::debug;
use pea_core::{
    constants::{BLOCK_TIME_MIN, MICROS, MIN_STAKE, NANOS, SYNC_BLOCKS_PER_TICK, TPS},
    types, util,
};
use pea_stake::Stake;
use std::time::{Duration, SystemTime};
pub async fn next() {
    let mut nanos = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let secs = nanos / NANOS;
    nanos -= secs * NANOS;
    nanos = NANOS - nanos;
    tokio::time::sleep(Duration::from_nanos(nanos as u64)).await
}
pub fn handler(swarm: &mut Swarm<MyBehaviour>) {
    let behaviour = swarm.behaviour_mut();
    behaviour.heartbeats += 1;
    sync(behaviour);
    if behaviour.heartbeats % TPS != 0 {
        return;
    }
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
    let data = bincode::serialize(&block).unwrap();
    if !behaviour.filter(&data, true) && behaviour.gossipsub.all_peers().count() > 0 {
        behaviour.gossipsub.publish(IdentTopic::new("block"), data).unwrap();
    }
}
fn sync(behaviour: &mut MyBehaviour) {
    if behaviour.blockchain.states.dynamic.hashes.is_empty() {
        return;
    }
    if behaviour.gossipsub.all_peers().count() == 0 {
        behaviour.blockchain.sync.index = 0;
        return;
    }
    for _ in 0..SYNC_BLOCKS_PER_TICK {
        let block = behaviour.blockchain.sync_block();
        let data = bincode::serialize(&block).unwrap();
        let _ = behaviour.gossipsub.publish(IdentTopic::new("block"), data);
    }
}
fn lag(behaviour: &mut MyBehaviour) {
    let mut micros = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
    let secs = micros / MICROS;
    micros -= secs * MICROS;
    let millis = micros as f64 / 1_000_f64;
    behaviour.lag = millis;
    debug!("{} {} {}ms", "Heartbeat".cyan(), behaviour.heartbeats, millis.to_string().yellow(),);
}
