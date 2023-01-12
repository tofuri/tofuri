use crate::{
    node::Node,
    p2p::{self, FileRequest},
    util,
};
use colored::*;
use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};
use log::{debug, info, warn};
use pea_address::address;
use pea_block::BlockA;
use pea_core::*;
use std::time::Duration;
fn delay(node: &mut Node, seconds: usize) -> bool {
    (node.heartbeats as f64 % (node.tps * seconds as f64)) as usize == 0
}
pub fn handler(node: &mut Node, instant: tokio::time::Instant) {
    let timestamp = util::timestamp();
    if delay(node, 60) {
        dial_known(node);
    }
    if delay(node, 10) {
        share(node);
    }
    if delay(node, 5) {
        dial_unknown(node);
        let peer_ids = node.p2p_swarm.connected_peers().cloned().collect::<Vec<PeerId>>();
        let behaviour = node.p2p_swarm.behaviour_mut();
        for peer_id in peer_ids {
            behaviour.request_response.send_request(&peer_id, FileRequest(vec![0, 1, 2, 3, 4, 5]));
        }
    }
    if delay(node, 2) {
        node.p2p_message_data_hashes.clear();
    }
    if delay(node, 1) {
        node.blockchain.sync.handler();
        node.p2p_ratelimit.reset();
    }
    pending_blocks(node);
    offline_staker(node, timestamp);
    grow(node, timestamp);
    sync(node);
    node.heartbeats += 1;
    lag(node, instant.elapsed());
}
fn pending_blocks(node: &mut Node) {
    let drain = node.blockchain.pending_blocks.drain(..);
    let mut vec: Vec<BlockA> = vec![];
    for block in drain {
        if vec.iter().any(|a| a.hash == block.hash) {
            continue;
        }
        vec.push(block);
    }
    loop {
        if let Some(block) = vec.iter().find(|&block_a| match node.blockchain.validate_block_1(&block_a) {
            Ok(()) => true,
            Err(err) => {
                debug!("{}", err);
                false
            }
        }) {
            node.blockchain.accept_block(&block, false);
        } else {
            break;
        }
    }
}
fn offline_staker(node: &mut Node, timestamp: u32) {
    if node.p2p_ban_offline == 0 {
        return;
    }
    if !node.blockchain.sync.completed {
        return;
    }
    if node.p2p_connections.len() < node.p2p_ban_offline {
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
    let vec = node.p2p_known.clone().into_iter().collect();
    dial(node, vec, true);
}
fn dial_unknown(node: &mut Node) {
    let vec = node.p2p_unknown.drain().collect();
    dial(node, vec, false);
}
fn dial(node: &mut Node, vec: Vec<Multiaddr>, known: bool) {
    for mut multiaddr in vec {
        if node
            .p2p_connections
            .contains_key(&p2p::multiaddr_filter_ip(&multiaddr).expect("multiaddr to include ip"))
        {
            continue;
        }
        let addr = p2p::multiaddr_addr(&multiaddr).expect("multiaddr to include ip");
        if node.p2p_ratelimit.is_ratelimited(&node.p2p_ratelimit.get(&addr).1) {
            continue;
        }
        debug!(
            "Dialing {} peer {}",
            if known { "known".green() } else { "unknown".red() },
            multiaddr.to_string().magenta()
        );
        if !p2p::multiaddr_has_port(&multiaddr) {
            multiaddr.push(Protocol::Tcp(9333));
        }
        let _ = node.p2p_swarm.dial(multiaddr);
    }
}
fn share(node: &mut Node) {
    if !node.gossipsub_has_mesh_peers("multiaddr") {
        return;
    }
    let vec: Vec<&Multiaddr> = node.p2p_connections.keys().collect();
    node.gossipsub_publish("multiaddr", bincode::serialize(&vec).unwrap());
}
fn grow(node: &mut Node, timestamp: u32) {
    if !node.blockchain.sync.downloading() && !node.mint && node.blockchain.states.dynamic.next_staker(timestamp).is_none() {
        if delay(node, 60) {
            info!(
                "Waiting for synchronization to start... Currently connected to {} peers.",
                node.p2p_connections.len().to_string().yellow()
            );
        }
        node.blockchain.sync.completed = false;
    }
    if !node.blockchain.sync.completed {
        return;
    }
    if let Some(block) = node.blockchain.forge_block(timestamp) {
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
fn lag(node: &mut Node, duration: Duration) {
    node.lag = duration.as_micros() as f64 / 1_000_f64;
    debug!("{} {} {}", "Heartbeat".cyan(), node.heartbeats, format!("{:?}", duration).yellow());
}
