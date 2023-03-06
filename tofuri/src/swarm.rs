use crate::Node;
use colored::*;
use libp2p::core::connection::ConnectedPoint;
use libp2p::core::either::EitherError;
use libp2p::gossipsub::error::GossipsubHandlerError;
use libp2p::gossipsub::GossipsubEvent;
use libp2p::gossipsub::GossipsubMessage;
use libp2p::mdns;
use libp2p::request_response::RequestResponseEvent;
use libp2p::request_response::RequestResponseMessage;
use libp2p::request_response::ResponseChannel;
use libp2p::swarm::ConnectionHandlerUpgrErr;
use libp2p::swarm::SwarmEvent;
use libp2p::Multiaddr;
use libp2p::PeerId;
use std::error::Error;
use std::io;
use std::num::NonZeroU32;
use tofuri_block::BlockB;
use tofuri_core::*;
use tofuri_db as db;
use tofuri_p2p::behaviour::OutEvent;
use tofuri_p2p::behaviour::SyncRequest;
use tofuri_p2p::behaviour::SyncResponse;
use tofuri_p2p::multiaddr;
use tofuri_p2p::ratelimit::Endpoint;
use tofuri_stake::StakeB;
use tofuri_transaction::TransactionB;
use tracing::error;
use tracing::info;
use tracing::warn;
use void::Void;
type HandlerErr = EitherError<
    EitherError<EitherError<EitherError<Void, io::Error>, GossipsubHandlerError>, ConnectionHandlerUpgrErr<io::Error>>,
    ConnectionHandlerUpgrErr<io::Error>,
>;
#[tracing::instrument(skip_all, level = "trace")]
pub fn event(node: &mut Node, event: SwarmEvent<OutEvent, HandlerErr>) {
    match event {
        SwarmEvent::Dialing(_) => {}
        SwarmEvent::IncomingConnectionError { .. } => {}
        SwarmEvent::IncomingConnection { .. } => {}
        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint,
            num_established,
            ..
        } => {
            connection_established(node, peer_id, endpoint, num_established);
        }
        SwarmEvent::ConnectionClosed { endpoint, num_established, .. } => {
            connection_closed(node, endpoint, num_established);
        }
        SwarmEvent::Behaviour(OutEvent::Mdns(mdns::Event::Discovered(list))) => {
            for (_, multiaddr) in list {
                if let Some(multiaddr) = multiaddr::ip_port(&multiaddr) {
                    node.p2p.unknown.insert(multiaddr);
                }
            }
        }
        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
            message, propagation_source, ..
        })) => {
            if let Err(err) = gossipsub_message(node, message, propagation_source) {
                error!(err)
            }
        }
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { message, peer })) => match message {
            RequestResponseMessage::Request { request, channel, .. } => {
                if let Err(err) = sync_request(node, peer, request, channel) {
                    error!(err)
                }
            }
            RequestResponseMessage::Response { response, .. } => {
                if let Err(err) = sync_response(node, peer, response) {
                    error!(err)
                }
            }
        },
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::InboundFailure { .. })) => {}
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::OutboundFailure { .. })) => {}
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::ResponseSent { .. })) => {}
        _ => {}
    };
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_established(node: &mut Node, peer_id: PeerId, endpoint: ConnectedPoint, num_established: NonZeroU32) {
    if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
        if let Some(multiaddr) = multiaddr::ip_port(&address) {
            connection_established_save(node, peer_id, num_established, multiaddr);
        }
    }
    if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
        if let Some(multiaddr) = multiaddr::ip(&send_back_addr) {
            connection_established_save(node, peer_id, num_established, multiaddr);
        }
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_established_save(node: &mut Node, peer_id: PeerId, num_established: NonZeroU32, multiaddr: Multiaddr) {
    info!(multiaddr = multiaddr.to_string(), num_established, "Connection established");
    let addr = multiaddr::ip_addr(&multiaddr).expect("multiaddr to include ip");
    if node.p2p.ratelimit.is_ratelimited(&node.p2p.ratelimit.get(&addr).1) {
        warn!(multiaddr = multiaddr.to_string(), "Ratelimited");
        let _ = node.p2p.swarm.disconnect_peer_id(peer_id);
    }
    node.p2p.known.insert(multiaddr.clone());
    let _ = db::peer::put(&multiaddr.to_string(), &node.db);
    if let Some(previous_peer_id) = node
        .p2p
        .connections
        .insert(multiaddr::ip(&multiaddr).expect("multiaddr to include ip"), peer_id)
    {
        if previous_peer_id != peer_id {
            let _ = node.p2p.swarm.disconnect_peer_id(previous_peer_id);
        }
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_closed(node: &mut Node, endpoint: ConnectedPoint, num_established: u32) {
    if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
        if let Some(multiaddr) = multiaddr::ip_port(&address) {
            connection_closed_save(node, num_established, multiaddr);
        }
    }
    if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
        if let Some(multiaddr) = multiaddr::ip(&send_back_addr) {
            connection_closed_save(node, num_established, multiaddr);
        }
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_closed_save(node: &mut Node, num_established: u32, multiaddr: Multiaddr) {
    info!(multiaddr = multiaddr.to_string(), num_established, "Connection closed");
    node.p2p.connections.remove(&multiaddr);
    let _ = node.p2p.swarm.dial(multiaddr);
}
#[tracing::instrument(skip_all, level = "trace")]
fn gossipsub_message(node: &mut Node, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn std::error::Error>> {
    match message.topic.as_str() {
        "block" => {
            node.p2p.ratelimit(propagation_source, Endpoint::Block)?;
            if node.p2p.filter(&message.data) {
                return Err("filter block".into());
            }
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_blocks_push(&node.db, block_b, node.args.time_delta, node.args.trust)?;
            node.blockchain.save_blocks(&node.db, node.args.trust);
        }
        "transaction" => {
            node.p2p.ratelimit(propagation_source, Endpoint::Transaction)?;
            if node.p2p.filter(&message.data) {
                return Err("filter transaction".into());
            }
            let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction_b, node.args.time_delta)?;
        }
        "stake" => {
            node.p2p.ratelimit(propagation_source, Endpoint::Stake)?;
            if node.p2p.filter(&message.data) {
                return Err("filter stake".into());
            }
            let stake_b: StakeB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake_b, node.args.time_delta)?;
        }
        "multiaddr" => {
            node.p2p.ratelimit(propagation_source, Endpoint::Multiaddr)?;
            if node.p2p.filter(&message.data) {
                return Err("filter multiaddr".into());
            }
            for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                if let Some(multiaddr) = multiaddr::ip_port(&multiaddr) {
                    node.p2p.unknown.insert(multiaddr);
                }
            }
        }
        _ => {}
    };
    Ok(())
}
#[tracing::instrument(skip_all, level = "trace")]
fn sync_request(node: &mut Node, peer_id: PeerId, request: SyncRequest, channel: ResponseChannel<SyncResponse>) -> Result<(), Box<dyn Error>> {
    node.p2p.ratelimit(peer_id, Endpoint::SyncRequest)?;
    let height: usize = bincode::deserialize(&request.0)?;
    let mut vec = vec![];
    for i in 0..SYNC_BLOCKS {
        match node.blockchain.sync_block(&node.db, height + i) {
            Some(block_b) => vec.push(block_b),
            None => break,
        }
    }
    if node
        .p2p
        .swarm
        .behaviour_mut()
        .request_response
        .send_response(channel, SyncResponse(bincode::serialize(&vec).unwrap()))
        .is_err()
    {
        return Err("p2p request handler connection closed".into());
    };
    Ok(())
}
#[tracing::instrument(skip_all, level = "trace")]
fn sync_response(node: &mut Node, peer_id: PeerId, response: SyncResponse) -> Result<(), Box<dyn Error>> {
    node.p2p.ratelimit(peer_id, Endpoint::SyncResponse)?;
    for block_b in bincode::deserialize::<Vec<BlockB>>(&response.0)? {
        node.blockchain.pending_blocks_push(&node.db, block_b, node.args.time_delta, node.args.trust)?;
        node.blockchain.save_blocks(&node.db, node.args.trust);
    }
    Ok(())
}
