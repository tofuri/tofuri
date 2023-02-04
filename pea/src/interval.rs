use crate::Node;
use colored::*;
use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use log::debug;
use log::error;
use log::info;
use log::warn;
use pea_address::address;
use pea_p2p::behaviour::SyncRequest;
use pea_p2p::multiaddr;
use pea_util;
use rand::prelude::*;
use std::time::Duration;
pub fn tick(node: &mut Node, instant: tokio::time::Instant) {
    let timestamp = pea_util::timestamp();
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
        node.blockchain.sync.handler();
        node.p2p.ratelimit.reset();
        node.p2p.filter.clear();
    }
    sync_request(node);
    offline_staker(node, timestamp);
    grow(node, timestamp);
    node.ticks += 1;
    lag(node, instant.elapsed());
}
fn delay(node: &Node, seconds: usize) -> bool {
    (node.ticks as f64 % (node.args.tps * seconds as f64)) as usize == 0
}
fn offline_staker(node: &mut Node, timestamp: u32) {
    if node.p2p.ban_offline == 0 {
        return;
    }
    if !node.blockchain.sync.completed {
        return;
    }
    if node.p2p.connections.len() < node.p2p.ban_offline {
        return;
    }
    let dynamic = &node.blockchain.states.dynamic;
    for staker in dynamic.stakers_offline(timestamp, dynamic.latest_block.timestamp) {
        if let Some(hash) = node.blockchain.offline.insert(staker, dynamic.latest_block.hash) {
            if hash == dynamic.latest_block.hash {
                return;
            }
        }
        warn!("Banned offline staker {}", address::encode(&staker).green());
    }
}
fn dial_known(node: &mut Node) {
    let vec = node.p2p.known.clone().into_iter().collect();
    dial(node, vec, true);
}
fn dial_unknown(node: &mut Node) {
    let vec = node.p2p.unknown.drain().collect();
    dial(node, vec, false);
}
fn dial(node: &mut Node, vec: Vec<Multiaddr>, known: bool) {
    for mut multiaddr in vec {
        if node.p2p.connections.contains_key(&multiaddr::ip(&multiaddr).expect("multiaddr to include ip")) {
            continue;
        }
        let addr = multiaddr::ip_addr(&multiaddr).expect("multiaddr to include ip");
        if node.p2p.ratelimit.is_ratelimited(&node.p2p.ratelimit.get(&addr).1) {
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
        let _ = node.p2p.swarm.dial(multiaddr);
    }
}
fn share(node: &mut Node) {
    if !node.p2p.gossipsub_has_mesh_peers("multiaddr") {
        return;
    }
    let vec: Vec<&Multiaddr> = node.p2p.connections.keys().collect();
    if let Err(err) = node.p2p.gossipsub_publish("multiaddr", bincode::serialize(&vec).unwrap()) {
        error!("{}", err);
    }
}
fn grow(node: &mut Node, timestamp: u32) {
    if !node.blockchain.sync.downloading() && !node.args.mint && node.blockchain.states.dynamic.next_staker(timestamp).is_none() {
        if delay(node, 60) {
            info!(
                "Waiting for synchronization to start... Currently connected to {} peers.",
                node.p2p.connections.len().to_string().yellow()
            );
        }
        node.blockchain.sync.completed = false;
    }
    if !node.blockchain.sync.completed {
        return;
    }
    if let Some(block_a) = node.blockchain.forge_block(&node.db, &node.key, timestamp) {
        if !node.p2p.gossipsub_has_mesh_peers("block") {
            return;
        }
        if let Err(err) = node.p2p.gossipsub_publish("block", bincode::serialize(&block_a.b()).unwrap()) {
            error!("{}", err);
        }
    }
}
fn sync_request(node: &mut Node) {
    if let Some(peer_id) = node.p2p.swarm.connected_peers().choose(&mut thread_rng()).cloned() {
        node.p2p
            .swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer_id, SyncRequest(bincode::serialize(&(node.blockchain.height())).unwrap()));
    }
}
fn lag(node: &mut Node, duration: Duration) {
    node.lag = duration.as_micros() as f64 / 1_000_f64;
    debug!("{} {} {}", "Tick".cyan(), node.ticks, format!("{duration:?}").yellow());
}
