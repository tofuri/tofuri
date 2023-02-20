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
use log::debug;
use log::error;
use log::info;
use log::warn;
use pea_block::BlockB;
use pea_core::*;
use pea_db as db;
use pea_p2p::behaviour::OutEvent;
use pea_p2p::behaviour::SyncRequest;
use pea_p2p::behaviour::SyncResponse;
use pea_p2p::multiaddr;
use pea_p2p::ratelimit::Endpoint;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::error::Error;
use std::io;
use std::num::NonZeroU32;
use tokio::time::Instant;
use void::Void;
type HandlerErr = EitherError<
    EitherError<EitherError<EitherError<Void, io::Error>, GossipsubHandlerError>, ConnectionHandlerUpgrErr<io::Error>>,
    ConnectionHandlerUpgrErr<io::Error>,
>;
pub fn event(node: &mut Node, event: SwarmEvent<OutEvent, HandlerErr>) -> Instant {
    let instant = Instant::now();
    debug!("{:?}", event);
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
            event_connection_established(node, peer_id, endpoint, num_established);
        }
        SwarmEvent::ConnectionClosed { endpoint, num_established, .. } => {
            event_connection_closed(node, endpoint, num_established);
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
            if let Err(err) = event_gossipsub_message(node, message, propagation_source) {
                error!("GossipsubEvent::Message {}", err)
            }
        }
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { message, peer })) => match message {
            RequestResponseMessage::Request { request, channel, .. } => {
                if let Err(err) = event_request(node, peer, request, channel) {
                    error!("RequestResponseMessage::Request {}", err)
                }
            }
            RequestResponseMessage::Response { response, .. } => {
                if let Err(err) = event_response(node, peer, response) {
                    error!("RequestResponseMessage::Response {}", err)
                }
            }
        },
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::InboundFailure { .. })) => {}
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::OutboundFailure { .. })) => {}
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::ResponseSent { .. })) => {}
        _ => {}
    };
    instant
}
fn event_connection_established(node: &mut Node, peer_id: PeerId, endpoint: ConnectedPoint, num_established: NonZeroU32) {
    if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
        if let Some(multiaddr) = multiaddr::ip_port(&address) {
            event_connection_established_save(node, peer_id, num_established, multiaddr);
        }
    }
    if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
        if let Some(multiaddr) = multiaddr::ip(&send_back_addr) {
            event_connection_established_save(node, peer_id, num_established, multiaddr);
        }
    }
}
fn event_connection_established_save(node: &mut Node, peer_id: PeerId, num_established: NonZeroU32, multiaddr: Multiaddr) {
    info!(
        "Connection {} {} {}",
        "established".green(),
        multiaddr.to_string().magenta(),
        num_established.to_string().yellow()
    );
    let addr = multiaddr::ip_addr(&multiaddr).expect("multiaddr to include ip");
    if node.p2p.ratelimit.is_ratelimited(&node.p2p.ratelimit.get(&addr).1) {
        warn!("Ratelimited {}", multiaddr.to_string().magenta());
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
fn event_connection_closed(node: &mut Node, endpoint: ConnectedPoint, num_established: u32) {
    if let ConnectedPoint::Dialer { address, .. } = endpoint.clone() {
        if let Some(multiaddr) = multiaddr::ip_port(&address) {
            event_connection_closed_save(node, num_established, multiaddr);
        }
    }
    if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
        if let Some(multiaddr) = multiaddr::ip(&send_back_addr) {
            event_connection_closed_save(node, num_established, multiaddr);
        }
    }
}
fn event_connection_closed_save(node: &mut Node, num_established: u32, multiaddr: Multiaddr) {
    info!(
        "Connection {} {} {}",
        "closed".red(),
        multiaddr.to_string().magenta(),
        num_established.to_string().yellow()
    );
    node.p2p.connections.remove(&multiaddr);
    let _ = node.p2p.swarm.dial(multiaddr);
}
fn event_gossipsub_message(node: &mut Node, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn std::error::Error>> {
    match message.topic.as_str() {
        "block" => {
            node.p2p.ratelimit(propagation_source, Endpoint::Block)?;
            if node.p2p.filter(&message.data) {
                return Err("filter block".into());
            }
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_blocks_push(&node.db, block_b, node.args.time_delta, node.args.trust)?;
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
fn event_request(node: &mut Node, peer_id: PeerId, request: SyncRequest, channel: ResponseChannel<SyncResponse>) -> Result<(), Box<dyn Error>> {
    node.p2p.ratelimit(peer_id, Endpoint::SyncRequest)?;
    let height: usize = bincode::deserialize(&request.0)?;
    let mut vec = vec![];
    for i in 0..SYNC_BLOCKS_PER_TICK {
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
fn event_response(node: &mut Node, peer_id: PeerId, response: SyncResponse) -> Result<(), Box<dyn Error>> {
    node.p2p.ratelimit(peer_id, Endpoint::SyncResponse)?;
    for block_b in bincode::deserialize::<Vec<BlockB>>(&response.0)? {
        match node.blockchain.pending_blocks_push(&node.db, block_b, node.args.time_delta, node.args.trust) {
            Ok(()) => node.blockchain.save_blocks(&node.db, node.args.trust),
            Err(err) => debug!("response_handler {}", err),
        }
    }
    Ok(())
}
