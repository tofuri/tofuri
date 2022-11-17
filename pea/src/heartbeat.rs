use crate::node::Node;
use colored::*;
use libp2p::{gossipsub::IdentTopic, multiaddr::Protocol, Multiaddr};
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
pub fn handler(node: &mut Node) {
    node.heartbeats += 1;
    share_peer_list(node);
    dial_new_multiaddrs(node);
    sync(node);
    node.message_data_hashes.clear();
    node.blockchain.sync.handler();
    forge(node);
    node.blockchain.pending_blocks_accept();
    lag(node);
}
fn dial_new_multiaddrs(node: &mut Node) {
    let new_multiaddrs = node.new_multiaddrs.clone();
    for mut multiaddr in new_multiaddrs {
        if node.peer_list.contains_key(&multiaddr) {
            continue;
        }
        multiaddr.push(Protocol::Tcp(9333));
        let _ = node.swarm.dial(multiaddr);
    }
    node.new_multiaddrs.clear();
}
fn share_peer_list(node: &mut Node) {
    if node.heartbeats % (node.tps * 10_f64) as usize != 0 {
        return;
    }
    if node.swarm.behaviour().gossipsub.all_peers().count() == 0 {
        return;
    }
    let vec: Vec<&Multiaddr> = node.peer_list.keys().collect();
    let data = bincode::serialize(&vec).unwrap();
    if let Err(err) = node.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("multiaddr"), data) {
        error!("{}", err);
    }
}
fn forge(node: &mut Node) {
    let states = &node.blockchain.states;
    if node.blockchain.sync.syncing {
        return;
    }
    let timestamp = util::timestamp();
    if let Some(public_key) = states.dynamic.staker(timestamp, states.dynamic.latest_block.timestamp) {
        if public_key != &node.blockchain.key.public_key_bytes() || timestamp < states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as u32 {
            return;
        }
    } else {
        let mut stake = Stake::new(true, MIN_STAKE, 0);
        stake.sign(&node.blockchain.key);
        node.blockchain.set_cold_start_stake(stake);
    }
    let block = node.blockchain.forge_block().unwrap();
    if node.swarm.behaviour().gossipsub.all_peers().count() == 0 {
        return;
    }
    let data = bincode::serialize(&block).unwrap();
    if node.filter(&data, true) {
        return;
    }
    if let Err(err) = node.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("block"), data) {
        error!("{}", err);
    }
}
fn sync(node: &mut Node) {
    if node.blockchain.states.dynamic.hashes.is_empty() {
        return;
    }
    if node.swarm.behaviour().gossipsub.all_peers().count() == 0 {
        node.blockchain.sync.index_0 = 0;
        return;
    }
    for _ in 0..SYNC_BLOCKS_PER_TICK {
        for block in node.blockchain.sync_blocks() {
            let data = bincode::serialize(&block).unwrap();
            if let Err(err) = node.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("block sync"), data) {
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
fn lag(node: &mut Node) {
    let f = 1_f64 / node.tps;
    let u = (f * 1_000_000_000_f64) as u64;
    let nanos = u - nanos(node.tps);
    node.lag = (nanos / 1_000) as f64 / 1_000_f64;
    debug!("{} {} {}", "Heartbeat".cyan(), node.heartbeats, format!("{:?}", Duration::from_nanos(nanos)).yellow());
}
