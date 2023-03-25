use crate::Node;
use rand::prelude::*;
use std::net::IpAddr;
use tofuri_core::*;
use tofuri_p2p::behaviour::SyncRequest;
use tofuri_p2p::multiaddr;
use tofuri_util;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
#[tracing::instrument(skip_all, level = "debug")]
pub fn interval_1s(node: &mut Node) {
    sync_request(node);
    node.blockchain.sync.handler();
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn interval_10s(node: &mut Node) {
    dial_known(node)
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn interval_1m(node: &mut Node) {
    grow(node);
    share(node);
    dial_unknown(node);
    node.p2p.request_response_counter.clear();
    node.p2p.gossipsub_message_counter_block.clear();
    node.p2p.gossipsub_message_counter_transaction.clear();
    node.p2p.gossipsub_message_counter_stake.clear();
    node.p2p.gossipsub_message_counter_peers.clear();
}
#[tracing::instrument(skip_all, level = "debug")]
pub fn interval_10m(node: &mut Node) {
    checkpoint(node);
}
#[tracing::instrument(skip_all, level = "debug")]
fn dial_known(node: &mut Node) {
    let vec = node.p2p.connections_known.clone().into_iter().collect();
    dial(node, vec);
}
#[tracing::instrument(skip_all, level = "debug")]
fn dial_unknown(node: &mut Node) {
    let vec = node.p2p.connections_unknown.drain().collect();
    dial(node, vec);
}
#[tracing::instrument(skip_all, level = "debug")]
fn dial(node: &mut Node, vec: Vec<IpAddr>) {
    for ip_addr in vec {
        if node.p2p.connections.iter().any(|x| x.1 == &ip_addr) {
            continue;
        }
        debug!(ip_addr = ip_addr.to_string(), "Dial");
        let _ = node.p2p.swarm.dial(multiaddr::from_ip_addr(&ip_addr));
    }
}
#[tracing::instrument(skip_all, level = "debug")]
fn share(node: &mut Node) {
    let vec: Vec<&IpAddr> = node.p2p.connections.values().collect();
    if vec.is_empty() {
        return;
    }
    debug!(connections = vec.len(), "Share");
    if let Err(err) = node
        .p2p
        .gossipsub_publish("peers", bincode::serialize(&vec).unwrap())
    {
        error!("{:?}", err);
    }
}
#[tracing::instrument(skip_all, level = "debug")]
fn grow(node: &mut Node) {
    let timestamp = tofuri_util::timestamp();
    let timestamp = timestamp - (timestamp % BLOCK_TIME);
    node.blockchain.pending_retain(timestamp);
    node.blockchain.save_blocks(&node.db, node.args.trust);
    if !node.blockchain.sync.downloading()
        && !node.args.mint
        && node
            .blockchain
            .forks
            .unstable
            .next_staker(timestamp)
            .is_none()
    {
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
    let block_a = node
        .blockchain
        .forge_block(&node.db, &node.key, timestamp, node.args.trust);
    if let Err(err) = node
        .p2p
        .gossipsub_publish("block", bincode::serialize(&block_a.b()).unwrap())
    {
        error!("{:?}", err);
    }
}
#[tracing::instrument(skip_all, level = "debug")]
fn sync_request(node: &mut Node) {
    if node.blockchain.forks.unstable.latest_block.timestamp
        >= tofuri_util::timestamp() - BLOCK_TIME
    {
        return;
    }
    if let Some(peer_id) = node
        .p2p
        .swarm
        .connected_peers()
        .choose(&mut thread_rng())
        .cloned()
    {
        node.p2p
            .swarm
            .behaviour_mut()
            .request_response
            .send_request(
                &peer_id,
                SyncRequest(bincode::serialize(&(node.blockchain.height())).unwrap()),
            );
    }
}
#[tracing::instrument(skip_all, level = "debug")]
fn checkpoint(node: &mut Node) {
    let checkpoint = node.blockchain.forks.stable.checkpoint();
    tofuri_db::checkpoint::put(&node.db, &checkpoint).unwrap();
    info!(checkpoint.height);
}
