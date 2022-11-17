use crate::{multiaddr, node::Node};
use colored::*;
use libp2p::{gossipsub::IdentTopic, multiaddr::Protocol, Multiaddr};
use log::{debug, error, info};
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
    dial_known(node);
    node.heartbeats += 1;
    dial_unknown(node);
    share(node);
    sync(node);
    node.message_data_hashes.clear();
    node.blockchain.sync.handler();
    forge(node);
    node.blockchain.pending_blocks_accept();
    lag(node);
}
fn dial_known(node: &mut Node) {
    if node.heartbeats % (node.tps * 60_f64) as usize != 0 {
        return;
    }
    let vec = node.known.clone().into_iter().collect();
    dial(node, vec, true);
}
fn dial_unknown(node: &mut Node) {
    if node.heartbeats % (node.tps * 60_f64) as usize != 0 {
        return;
    }
    let vec = node.unknown.drain().collect();
    dial(node, vec, false);
}
fn dial(node: &mut Node, vec: Vec<Multiaddr>, known: bool) {
    for mut multiaddr in vec {
        if node.connections.contains_key(&multiaddr) {
            continue;
        }
        info!("{} {} {}", "Dial".cyan(), if known { "known".green() } else { "unknown".red() }, multiaddr.to_string().magenta());
        if !multiaddr::has_port(&multiaddr) {
            multiaddr.push(Protocol::Tcp(9333));
        }
        let _ = node.swarm.dial(multiaddr);
    }
}
fn share(node: &mut Node) {
    if node.heartbeats % (node.tps * 60_f64) as usize != 0 {
        return;
    }
    if node.swarm.behaviour().gossipsub.all_peers().count() == 0 {
        return;
    }
    let vec: Vec<&Multiaddr> = node.connections.keys().collect();
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
