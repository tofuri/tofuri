use crate::{multiaddr, node::Node};
use colored::*;
use libp2p::{gossipsub::TopicHash, multiaddr::Protocol, Multiaddr};
use log::{debug, info, warn};
use pea_block::Block;
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
    if delay(node, 2) {
        node.message_data_hashes.clear();
    }
    if delay(node, 1) {
        node.blockchain.sync.handler();
    }
    pending_blocks(node);
    offline_staker(node);
    grow(node);
    sync(node);
    node.heartbeats += 1;
    lag(node);
}
fn pending_blocks(node: &mut Node) {
    let drain = node.blockchain.pending_blocks.drain(..);
    let mut vec: Vec<Block> = vec![];
    for block in drain {
        if vec.iter().any(|a| a.signature == block.signature) {
            continue;
        }
        vec.push(block);
    }
    loop {
        if let Some(block) = vec.iter().find(|a| node.blockchain.validate_block(a).is_ok()) {
            node.blockchain.accept_block(&block, false);
        } else {
            break;
        }
    }
}
fn offline_staker(node: &mut Node) {
    if node.ban_offline == 0 {
        return;
    }
    if node.blockchain.sync.downloading {
        return;
    }
    let behaviour = node.swarm.behaviour();
    if behaviour.gossipsub.mesh_peers(&TopicHash::from_raw("block")).count() < node.ban_offline {
        return;
    }
    let dynamic = &node.blockchain.states.dynamic;
    if let Some(public_key) = dynamic.offline_staker() {
        let latest_hash = dynamic.latest_block.hash();
        if let Some(hash) = node.blockchain.offline.insert(public_key.clone(), latest_hash) {
            if hash == latest_hash {
                return;
            }
        }
        warn!("Banned offline staker {}", pea_address::public::encode(&public_key).green());
    }
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
    if !node.gossipsub_has_mesh_peers("multiaddr") {
        return;
    }
    let vec: Vec<&Multiaddr> = node.connections.keys().collect();
    node.gossipsub_publish("multiaddr", bincode::serialize(&vec).unwrap());
}
fn grow(node: &mut Node) {
    if !node.blockchain.sync.downloading && !node.mint && node.blockchain.states.dynamic.current_staker().is_none() {
        if delay(node, 60) {
            info!(
                "Waiting for synchronization to start... Currently connected to {} peers.",
                node.swarm
                    .behaviour()
                    .gossipsub
                    .mesh_peers(&TopicHash::from_raw("blocks"))
                    .count()
                    .to_string()
                    .yellow()
            );
        }
        node.blockchain.sync.downloading = true;
    }
    if node.blockchain.sync.downloading {
        return;
    }
    if let Some(block) = node.blockchain.forge_block() {
        if !node.gossipsub_has_mesh_peers("block") {
            return;
        }
        node.gossipsub_publish("block", bincode::serialize(&block).unwrap());
    }
}
fn sync(node: &mut Node) {
    if node.blockchain.states.dynamic.hashes.is_empty() {
        return;
    }
    if !node.gossipsub_has_mesh_peers("blocks") {
        node.blockchain.sync.index = 0;
        return;
    }
    let mut vec = vec![];
    for _ in 0..SYNC_BLOCKS_PER_TICK {
        vec.push(node.blockchain.sync_block());
    }
    node.gossipsub_publish("blocks", bincode::serialize(&vec).unwrap())
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
