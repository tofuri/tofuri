use crate::{multiaddr, node::Node};
use colored::*;
use libp2p::{gossipsub::IdentTopic, multiaddr::Protocol, Multiaddr};
use log::{debug, error, info};
use pea_core::constants::SYNC_BLOCKS_PER_TICK;
use std::time::{Duration, SystemTime};
pub async fn next(tps: f64) {
    tokio::time::sleep(Duration::from_nanos(nanos(tps))).await
}
fn delay(node: &mut Node, seconds: usize) -> bool {
    node.heartbeats % (node.tps * seconds as f64) as usize == 0
}
pub fn handler(node: &mut Node) {
    if delay(node, 60) {
        dial_known(node);
    }
    if delay(node, 10) {
        share(node);
    }
    if delay(node, 5) {
        dial_unknown(node);
    }
    if delay(node, 1) {
        node.message_data_hashes.clear();
        node.blockchain.sync.handler();
    }
    node.blockchain.accept_pending_blocks();
    grow(node);
    sync(node);
    node.heartbeats += 1;
    lag(node);
}
fn dial_known(node: &mut Node) {
    let vec = node.known.clone().into_iter().collect();
    dial(node, vec, true);
}
fn dial_unknown(node: &mut Node) {
    let vec = node.unknown.drain().collect();
    dial(node, vec, false);
}
fn dial(node: &mut Node, vec: Vec<Multiaddr>, known: bool) {
    for mut multiaddr in vec {
        if node
            .connections
            .contains_key(&multiaddr::filter_ip(&multiaddr).expect("multiaddr to include ip"))
        {
            continue;
        }
        debug!(
            "Dialing {} peer {}",
            if known { "known".green() } else { "unknown".red() },
            multiaddr.to_string().magenta()
        );
        if !multiaddr::has_port(&multiaddr) {
            multiaddr.push(Protocol::Tcp(9333));
        }
        let _ = node.swarm.dial(multiaddr);
    }
}
fn share(node: &mut Node) {
    if node.swarm.behaviour().gossipsub.all_peers().count() == 0 {
        return;
    }
    let vec: Vec<&Multiaddr> = node.connections.keys().collect();
    let data = bincode::serialize(&vec).unwrap();
    if node.filter(&data, true) {
        return;
    }
    if let Err(err) = node.swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("multiaddr"), data) {
        error!("{}", err);
    }
}
fn grow(node: &mut Node) {
    if node.blockchain.sync.syncing {
        return;
    }
    if !node.genesis && node.blockchain.height() == 0 {
        if delay(node, 3) {
            info!(
                "Waiting for synchronization to start... Currently connected to {} peers.",
                node.swarm.behaviour().gossipsub.all_mesh_peers().count().to_string().yellow()
            );
        }
        return;
    }
    if let Some(block) = node.blockchain.forge_block() {
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
    debug!(
        "{} {} {}",
        "Heartbeat".cyan(),
        node.heartbeats,
        format!("{:?}", Duration::from_nanos(nanos)).yellow()
    );
}
