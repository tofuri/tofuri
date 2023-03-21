use crate::Node;
use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use rand::prelude::*;
use tofuri_core::*;
use tofuri_p2p::behaviour::SyncRequest;
use tofuri_p2p::multiaddr;
use tofuri_util;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
#[tracing::instrument(skip_all, level = "debug")]
pub fn dial_known(node: &mut Node) {
    let vec = node.p2p.known.clone().into_iter().collect();
    dial(node, vec);
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn dial_unknown(node: &mut Node) {
    let vec = node.p2p.unknown.drain().collect();
    dial(node, vec);
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn clear(node: &mut Node) {
    node.blockchain.sync.handler();
    node.p2p.ratelimit.reset();
    node.p2p.filter.clear();
}
#[tracing::instrument(skip_all, level = "debug")]
fn dial(node: &mut Node, vec: Vec<Multiaddr>) {
    for mut multiaddr in vec {
        if node.p2p.connections.contains_key(&multiaddr::ip(&multiaddr).expect("multiaddr to include ip")) {
            continue;
        }
        let addr = multiaddr::ip_addr(&multiaddr).expect("multiaddr to include ip");
        if node.p2p.ratelimit.is_ratelimited(&node.p2p.ratelimit.get(&addr).1) {
            debug!(multiaddr = multiaddr.to_string(), "Dial skipped");
            continue;
        }
        debug!(multiaddr = multiaddr.to_string(), "Dial");
        if !multiaddr::has_port(&multiaddr) {
            multiaddr.push(Protocol::Tcp(9333));
        }
        let _ = node.p2p.swarm.dial(multiaddr);
    }
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn share(node: &mut Node) {
    let vec: Vec<&Multiaddr> = node.p2p.connections.keys().collect();
    if vec.is_empty() {
        return;
    }
    debug!(connections = vec.len(), "Share");
    if let Err(err) = node.p2p.gossipsub_publish("multiaddr", bincode::serialize(&vec).unwrap()) {
        error!(err);
    }
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn grow(node: &mut Node) {
    let timestamp = tofuri_util::timestamp();
    let timestamp = timestamp - (timestamp % BLOCK_TIME);
    node.blockchain.pending_retain(timestamp);
    node.blockchain.save_blocks(&node.db, node.args.trust);
    if !node.blockchain.sync.downloading() && !node.args.mint && node.blockchain.forks.unstable.next_staker(timestamp).is_none() {
        info!("Idling");
        node.blockchain.sync.completed = false;
    }
    if !node.blockchain.sync.completed {
        return;
    }
    if let Some(staker) = node.blockchain.forks.unstable.next_staker(timestamp) {
        if staker != node.key.address_bytes() {
            return;
        }
    } else {
        warn!("No stakers");
    }
    let block_a = node.blockchain.forge_block(&node.db, &node.key, timestamp, node.args.trust);
    if let Err(err) = node.p2p.gossipsub_publish("block", bincode::serialize(&block_a.b()).unwrap()) {
        error!(err);
    }
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn sync_request(node: &mut Node) {
    if node.blockchain.forks.unstable.latest_block.timestamp >= tofuri_util::timestamp() - BLOCK_TIME {
        return;
    }
    if let Some(peer_id) = node.p2p.swarm.connected_peers().choose(&mut thread_rng()).cloned() {
        node.p2p
            .swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer_id, SyncRequest(bincode::serialize(&(node.blockchain.height())).unwrap()));
    }
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn checkpoint(node: &mut Node) {
    let checkpoint = node.blockchain.forks.stable.checkpoint();
    tofuri_db::checkpoint::put(&node.db, &checkpoint).unwrap();
    info!(checkpoint.height);
}
