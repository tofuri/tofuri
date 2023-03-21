use crate::Node;
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
use libp2p::PeerId;
use std::error::Error;
use std::io;
use std::net::IpAddr;
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
#[tracing::instrument(skip_all, level = "debug")]
pub fn event(node: &mut Node, event: SwarmEvent<OutEvent, HandlerErr>) {
    if let Err(err) = match event {
        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint,
            num_established,
            ..
        } => connection_established(node, peer_id, endpoint, num_established),
        SwarmEvent::ConnectionClosed { peer_id, num_established, .. } => connection_closed(node, peer_id, num_established),
        SwarmEvent::Behaviour(OutEvent::Mdns(event)) => mdns(node, event),
        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
            message, propagation_source, ..
        })) => gossipsub_message(node, message, propagation_source),
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { message, peer })) => match message {
            RequestResponseMessage::Request { request, channel, .. } => sync_request(node, peer, request, channel),
            RequestResponseMessage::Response { response, .. } => sync_response(node, peer, response),
        },
        _ => Ok(()),
    } {
        error!(err)
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_established(node: &mut Node, peer_id: PeerId, endpoint: ConnectedPoint, num_established: NonZeroU32) -> Result<(), Box<dyn Error>> {
    let ip_addr = match endpoint {
        ConnectedPoint::Dialer { address, .. } => multiaddr::to_ip_addr(&address).unwrap(),
        ConnectedPoint::Listener { send_back_addr, .. } => multiaddr::to_ip_addr(&send_back_addr).unwrap(),
    };
    if node.p2p.ratelimit.is_ratelimited(&node.p2p.ratelimit.get(&ip_addr).1) {
        warn!(ip_addr = ip_addr.to_string(), "Ratelimited");
        let _ = node.p2p.swarm.disconnect_peer_id(peer_id);
    }
    node.p2p.known.insert(ip_addr);
    let _ = db::peer::put(&ip_addr, &node.db);
    // if let Some((previous_peer_id, _)) = node.p2p.connections.iter().find(|x| x.1 == &ip_addr) {
    // if previous_peer_id != &peer_id {
    // let _ = node.p2p.swarm.disconnect_peer_id(*previous_peer_id);
    // }
    // }
    node.p2p.connections.insert(peer_id, ip_addr);
    info!(ip_addr = ip_addr.to_string(), num_established, "Connection established");
    Ok(())
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_closed(node: &mut Node, peer_id: PeerId, num_established: u32) -> Result<(), Box<dyn Error>> {
    let ip_addr = node.p2p.connections.remove(&peer_id).unwrap();
    info!(ip_addr = ip_addr.to_string(), num_established, "Connection closed");
    Ok(())
}
#[tracing::instrument(skip_all, level = "trace")]
fn mdns(node: &mut Node, event: mdns::Event) -> Result<(), Box<dyn Error>> {
    match event {
        mdns::Event::Discovered(iter) => {
            for (_, multiaddr) in iter {
                let ip_addr = multiaddr::to_ip_addr(&multiaddr).unwrap();
                node.p2p.unknown.insert(ip_addr);
            }
        }
        _ => {}
    }
    Ok(())
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
        "peers" => {
            node.p2p.ratelimit(propagation_source, Endpoint::Peers)?;
            if node.p2p.filter(&message.data) {
                return Err("filter peers".into());
            }
            for ip_addr in bincode::deserialize::<Vec<IpAddr>>(&message.data)? {
                node.p2p.unknown.insert(ip_addr);
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
    let mut size = 0;
    let mut vec = vec![];
    loop {
        match node.blockchain.sync_block(&node.db, height + vec.len()) {
            Some(block_b) => {
                size += bincode::serialize(&block_b).unwrap().len();
                if size > MAX_TRANSMIT_SIZE {
                    break;
                }
                vec.push(block_b);
            }
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
        if let Err(err) = node.blockchain.pending_blocks_push(&node.db, block_b, node.args.time_delta, node.args.trust) {
            error!(err);
            break;
        }
        node.blockchain.save_blocks(&node.db, node.args.trust);
    }
    Ok(())
}
