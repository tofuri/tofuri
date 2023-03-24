use crate::Node;
use libp2p::core::connection::ConnectedPoint;
use libp2p::core::either::EitherError;
use libp2p::gossipsub::error::GossipsubHandlerError;
use libp2p::gossipsub::GossipsubEvent;
use libp2p::gossipsub::GossipsubMessage;
use libp2p::gossipsub::MessageAcceptance;
use libp2p::gossipsub::MessageId;
use libp2p::mdns;
use libp2p::request_response::RequestResponseEvent;
use libp2p::request_response::RequestResponseMessage;
use libp2p::request_response::ResponseChannel;
use libp2p::swarm::ConnectionHandlerUpgrErr;
use libp2p::swarm::SwarmEvent;
use libp2p::PeerId;
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
use tofuri_stake::StakeB;
use tofuri_transaction::TransactionB;
use tracing::debug;
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
    match event {
        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint,
            num_established,
            ..
        } => connection_established(node, peer_id, endpoint, num_established),
        SwarmEvent::ConnectionClosed { peer_id, num_established, .. } => connection_closed(node, peer_id, num_established),
        SwarmEvent::Behaviour(OutEvent::Mdns(event)) => mdns(node, event),
        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
            message_id,
            message,
            propagation_source,
            ..
        })) => gossipsub_message(node, message, message_id, propagation_source),
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { message, peer })) => match message {
            RequestResponseMessage::Request { request, channel, .. } => sync_request(node, peer, request, channel),
            RequestResponseMessage::Response { response, .. } => sync_response(node, peer, response),
        },
        _ => {}
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_established(node: &mut Node, peer_id: PeerId, endpoint: ConnectedPoint, num_established: NonZeroU32) {
    let ip_addr = match endpoint {
        ConnectedPoint::Dialer { address, .. } => multiaddr::to_ip_addr(&address).unwrap(),
        ConnectedPoint::Listener { send_back_addr, .. } => multiaddr::to_ip_addr(&send_back_addr).unwrap(),
    };
    node.p2p.known.insert(ip_addr);
    let _ = db::peer::put(&ip_addr, &node.db);
    // if let Some((previous_peer_id, _)) = node.p2p.connections.iter().find(|x| x.1 == &ip_addr) {
    // if previous_peer_id != &peer_id {
    // let _ = node.p2p.swarm.disconnect_peer_id(*previous_peer_id);
    // }
    // }
    node.p2p.connections.insert(peer_id, ip_addr);
    info!(ip_addr = ip_addr.to_string(), num_established, "Connection established");
}
#[tracing::instrument(skip_all, level = "trace")]
fn connection_closed(node: &mut Node, peer_id: PeerId, num_established: u32) {
    let ip_addr = node.p2p.connections.remove(&peer_id).unwrap();
    info!(ip_addr = ip_addr.to_string(), num_established, "Connection closed");
}
#[tracing::instrument(skip_all, level = "trace")]
fn mdns(node: &mut Node, event: mdns::Event) {
    match event {
        mdns::Event::Discovered(iter) => {
            for (_, multiaddr) in iter {
                let ip_addr = multiaddr::to_ip_addr(&multiaddr).unwrap();
                node.p2p.unknown.insert(ip_addr);
            }
        }
        _ => {}
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn gossipsub_message(node: &mut Node, message: GossipsubMessage, message_id: MessageId, propagation_source: PeerId) {
    #[derive(Debug)]
    enum Error {
        Bincode(bincode::Error),
        Blockchain(tofuri_blockchain::Error),
    }
    fn inner(node: &mut Node, message: GossipsubMessage) -> Result<(), Error> {
        match message.topic.as_str() {
            "block" => {
                let block_b: BlockB = bincode::deserialize(&message.data).map_err(Error::Bincode)?;
                node.blockchain
                    .pending_blocks_push(&node.db, block_b, node.args.time_delta, node.args.trust)
                    .map_err(Error::Blockchain)?;
                node.blockchain.save_blocks(&node.db, node.args.trust);
            }
            "transaction" => {
                let transaction_b: TransactionB = bincode::deserialize(&message.data).map_err(Error::Bincode)?;
                node.blockchain
                    .pending_transactions_push(transaction_b, node.args.time_delta)
                    .map_err(Error::Blockchain)?;
            }
            "stake" => {
                let stake_b: StakeB = bincode::deserialize(&message.data).map_err(Error::Bincode)?;
                node.blockchain.pending_stakes_push(stake_b, node.args.time_delta).map_err(Error::Blockchain)?;
            }
            "peers" => {
                for ip_addr in bincode::deserialize::<Vec<IpAddr>>(&message.data).map_err(Error::Bincode)? {
                    node.p2p.unknown.insert(ip_addr);
                }
            }
            _ => {}
        };
        Ok(())
    }
    if node.p2p.filter(&message.data) {
        return;
    }
    let res = match inner(node, message) {
        Ok(()) => {
            debug!("Gossipsub message processed");
            node.p2p
                .swarm
                .behaviour_mut()
                .gossipsub
                .report_message_validation_result(&message_id, &propagation_source, MessageAcceptance::Accept)
        }
        Err(Error::Blockchain(tofuri_blockchain::Error::BlockPending))
        | Err(Error::Blockchain(tofuri_blockchain::Error::BlockHashInTree))
        | Err(Error::Blockchain(tofuri_blockchain::Error::BlockPreviousHashNotInTree)) => node
            .p2p
            .swarm
            .behaviour_mut()
            .gossipsub
            .report_message_validation_result(&message_id, &propagation_source, MessageAcceptance::Ignore),
        Err(err) => {
            error!("{:?}", err);
            node.p2p
                .swarm
                .behaviour_mut()
                .gossipsub
                .report_message_validation_result(&message_id, &propagation_source, MessageAcceptance::Reject)
        }
    };
    match res {
        Ok(cache) => debug!(cache, "Message validation result reported"),
        Err(err) => error!("{:?}", err),
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn sync_request(node: &mut Node, peer_id: PeerId, request: SyncRequest, channel: ResponseChannel<SyncResponse>) {
    #[derive(Debug)]
    enum Error {
        Bincode(bincode::Error),
        Blockchain(tofuri_blockchain::Error),
        SyncResponse(tofuri_p2p::behaviour::SyncResponse),
    }
    fn inner(node: &mut Node, request: SyncRequest, channel: ResponseChannel<SyncResponse>) -> Result<(), Error> {
        let height: usize = bincode::deserialize(&request.0).map_err(Error::Bincode)?;
        let mut size = 0;
        let mut vec = vec![];
        loop {
            let block_b = node.blockchain.sync_block(&node.db, height + vec.len()).map_err(Error::Blockchain)?;
            size += bincode::serialize(&block_b).map_err(Error::Bincode)?.len();
            if size > MAX_TRANSMIT_SIZE {
                break;
            }
            vec.push(block_b);
        }
        let vec = bincode::serialize(&vec).map_err(Error::Bincode)?;
        node.p2p
            .swarm
            .behaviour_mut()
            .request_response
            .send_response(channel, SyncResponse(vec))
            .map_err(Error::SyncResponse)?;
        Ok(())
    }
    match inner(node, request, channel) {
        Ok(()) => debug!("Sync request processed"),
        Err(err) => {
            error!("{:?}", err);
            node.p2p.timeout(&peer_id);
        }
    }
}
#[tracing::instrument(skip_all, level = "trace")]
fn sync_response(node: &mut Node, peer_id: PeerId, response: SyncResponse) {
    #[derive(Debug)]
    enum Error {
        Bincode(bincode::Error),
        Blockchain(tofuri_blockchain::Error),
    }
    fn inner(node: &mut Node, response: SyncResponse) -> Result<(), Error> {
        for block_b in bincode::deserialize::<Vec<BlockB>>(&response.0).map_err(Error::Bincode)? {
            node.blockchain
                .pending_blocks_push(&node.db, block_b, node.args.time_delta, node.args.trust)
                .map_err(Error::Blockchain)?;
            node.blockchain.save_blocks(&node.db, node.args.trust);
        }
        Ok(())
    }
    match inner(node, response) {
        Ok(()) => debug!("Sync response processed"),
        Err(err) => {
            error!("{:?}", err);
            node.p2p.timeout(&peer_id);
        }
    }
}
