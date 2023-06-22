use crate::Node;
use crate::SHARE_PEERS_MAX_LEN;
use chrono::Utc;
use rand::prelude::*;
use std::net::IpAddr;
use tofuri_blockchain::fork::BLOCK_TIME;
use tofuri_p2p::behaviour::Request;
use tofuri_p2p::multiaddr;
use tofuri_p2p::ratelimit::Endpoint;
use tokio::time::Duration;
use tokio::time::Instant;
use tokio::time::Interval;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;
pub fn at(period: Duration) -> Interval {
    let nanos = period.as_nanos() as u64;
    let start = Instant::now()
        + Duration::from_nanos(nanos - chrono::offset::Utc::now().timestamp_nanos() as u64 % nanos);
    let mut interval = tokio::time::interval_at(start, period);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval
}
#[instrument(skip_all, level = "debug")]
pub fn interval_1s(node: &mut Node) {
    sync_request(node);
    node.blockchain.sync.handler();
}
#[instrument(skip_all, level = "debug")]
pub fn interval_10s(node: &mut Node) {
    dial_known(node)
}
#[instrument(skip_all, level = "debug")]
pub fn interval_1m(node: &mut Node) {
    grow(node);
    share(node);
    dial_unknown(node);
    node.p2p.ratelimit.counter.clear();
}
#[instrument(skip_all, level = "debug")]
pub fn interval_10m(node: &mut Node) {
    checkpoint(node);
}
#[instrument(skip_all, level = "debug")]
fn dial_known(node: &mut Node) {
    let vec = node.p2p.connections_known.clone().into_iter().collect();
    dial(node, vec);
}
#[instrument(skip_all, level = "debug")]
fn dial_unknown(node: &mut Node) {
    let vec = node.p2p.connections_unknown.drain().collect();
    dial(node, vec);
}
#[instrument(skip_all, level = "debug")]
fn dial(node: &mut Node, vec: Vec<IpAddr>) {
    for ip_addr in vec {
        if node.p2p.connections.iter().any(|x| x.1 == &ip_addr) {
            continue;
        }
        debug!(?ip_addr, "Dial");
        let _ = node
            .p2p
            .swarm
            .dial(multiaddr::from_ip_addr(&ip_addr, node.args.testnet));
    }
}
#[instrument(skip_all, level = "debug")]
fn share(node: &mut Node) {
    let mut vec: Vec<&IpAddr> = node.p2p.connections.values().collect();
    if vec.is_empty() {
        return;
    }
    vec.shuffle(&mut thread_rng());
    vec.truncate(SHARE_PEERS_MAX_LEN);
    debug!(?vec, "Share");
    if let Err(e) = node
        .p2p
        .gossipsub_publish("peers", bincode::serialize(&vec).unwrap())
    {
        error!(?e);
    }
}
#[instrument(skip_all, level = "debug")]
fn grow(node: &mut Node) {
    let timestamp = {
        let timestamp = Utc::now().timestamp() as u32;
        timestamp - (timestamp % BLOCK_TIME)
    };
    let blockchain = &mut node.blockchain;
    blockchain.pending_retain(timestamp);
    blockchain.save_blocks(&node.db, node.args.trust);
    let sync = &mut blockchain.sync;
    let unstable = &blockchain.forks.unstable;
    if !sync.downloading() && !node.args.mint && unstable.next_staker(timestamp).is_none() {
        info!("Idling");
        sync.completed = false;
    }
    if !sync.completed {
        return;
    }
    if !tofuri_blockchain::validate_block_timestamp(timestamp, unstable.latest_block.timestamp) {
        return;
    }
    let key = match &node.key {
        Some(x) => x,
        None => return debug!("No key, skip forge block"),
    };
    if let Some(staker) = unstable.next_staker(timestamp) {
        if staker != key.address_bytes() {
            return;
        }
    } else {
        warn!("No stakers");
    }
    let block = node
        .blockchain
        .forge_block(&node.db, key, timestamp, node.args.trust);
    if let Err(e) = node
        .p2p
        .gossipsub_publish("block", bincode::serialize(&block).unwrap())
    {
        error!(?e);
    }
}
#[instrument(skip_all, level = "debug")]
fn sync_request(node: &mut Node) {
    if node.blockchain.forks.unstable.latest_block.timestamp
        >= Utc::now().timestamp() as u32 - BLOCK_TIME
    {
        return;
    }
    let peer_id = match node.p2p.swarm.connected_peers().choose(&mut thread_rng()) {
        Some(x) => *x,
        None => return,
    };
    let ip_addr = match node.p2p.connections.get(&peer_id) {
        Some(x) => *x,
        None => return,
    };
    if node.p2p.ratelimit.timeout.has(ip_addr, Endpoint::Response) {
        return;
    }
    node.p2p
        .swarm
        .behaviour_mut()
        .request_response
        .send_request(
            &peer_id,
            Request(bincode::serialize(&(node.blockchain.height())).unwrap()),
        );
}
#[instrument(skip_all, level = "debug")]
fn checkpoint(node: &mut Node) {
    let checkpoint = node.blockchain.forks.stable.checkpoint();
    tofuri_db::checkpoint::put(&node.db, &checkpoint).unwrap();
    info!(checkpoint.height);
}
