use crate::Node;
use crate::SHARE_PEERS_MAX_LEN;
use block::Block;
use db;
use libp2p::core::connection::ConnectedPoint;
use libp2p::gossipsub;
use libp2p::gossipsub::MessageAcceptance;
use libp2p::gossipsub::MessageId;
use libp2p::mdns;
use libp2p::request_response;
use libp2p::request_response::ResponseChannel;
use libp2p::swarm::derive_prelude::Either;
use libp2p::swarm::ConnectionHandlerUpgrErr;
use libp2p::swarm::SwarmEvent;
use libp2p::PeerId;
use multiaddr::ToIpAddr;
use p2p::behaviour::OutEvent;
use p2p::behaviour::Request;
use p2p::behaviour::Response;
use p2p::ratelimit::Endpoint;
use p2p::MAX_TRANSMIT_SIZE;
use stake::Stake;
use std::io::Error;
use std::net::IpAddr;
use std::num::NonZeroU32;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;
use transaction::Transaction;
use void::Void;
type Event = SwarmEvent<
    OutEvent,
    Either<
        Either<
            Either<Either<Either<Void, Error>, Void>, ConnectionHandlerUpgrErr<Error>>,
            ConnectionHandlerUpgrErr<Error>,
        >,
        Void,
    >,
>;
#[instrument(skip_all, level = "debug")]
pub fn event(node: &mut Node, event: Event) {
    match event {
        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint,
            num_established,
            ..
        } => connection_established(node, peer_id, endpoint, num_established),
        SwarmEvent::ConnectionClosed {
            peer_id,
            num_established,
            ..
        } => connection_closed(node, peer_id, num_established),
        SwarmEvent::Behaviour(OutEvent::Mdns(event)) => mdns(node, event),
        SwarmEvent::Behaviour(OutEvent::Gossipsub(gossipsub::Event::Message {
            message_id,
            message,
            propagation_source,
            ..
        })) => gossipsub_message(node, message, message_id, propagation_source),
        SwarmEvent::Behaviour(OutEvent::RequestResponse(request_response::Event::Message {
            message,
            peer,
        })) => match message {
            request_response::Message::Request {
                request, channel, ..
            } => sync_request(node, peer, request, channel),
            request_response::Message::Response { response, .. } => {
                sync_response(node, peer, response)
            }
        },
        _ => {}
    }
}
#[instrument(skip_all, level = "trace")]
fn connection_established(
    node: &mut Node,
    peer_id: PeerId,
    endpoint: ConnectedPoint,
    num_established: NonZeroU32,
) {
    let ip_addr = match endpoint {
        ConnectedPoint::Dialer { address, .. } => address.ip_addr().unwrap(),
        ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr.ip_addr().unwrap(),
    };
    node.p2p.connections_known.insert(ip_addr);
    let _ = db::peer::put(&node.db, &ip_addr);
    // if let Some((previous_peer_id, _)) = node.p2p.connections.iter().find(|x| x.1 == &ip_addr) {
    // if previous_peer_id != &peer_id {
    // let _ = node.p2p.swarm.disconnect_peer_id(*previous_peer_id);
    // }
    // }
    node.p2p.connections.insert(peer_id, ip_addr);
    info!(?ip_addr, num_established, "Connection established");
}
#[instrument(skip_all, level = "trace")]
fn connection_closed(node: &mut Node, peer_id: PeerId, num_established: u32) {
    let res = node.p2p.connections.remove(&peer_id);
    let ip_addr = res.unwrap();
    info!(?ip_addr, num_established, "Connection closed");
}
#[instrument(skip_all, level = "trace")]
fn mdns(node: &mut Node, event: mdns::Event) {
    match event {
        mdns::Event::Discovered(iter) => {
            for (_, multiaddr) in iter {
                let ip_addr = multiaddr.ip_addr().unwrap();
                node.p2p.connections_unknown.insert(ip_addr);
            }
        }
        mdns::Event::Expired(_) => {}
    }
}
#[instrument(skip_all, level = "trace")]
fn gossipsub_message(
    node: &mut Node,
    message: gossipsub::Message,
    message_id: MessageId,
    propagation_source: PeerId,
) {
    #[derive(Debug)]
    enum Error {
        Bincode(bincode::Error),
        Blockchain(blockchain::Error),
        MessageSource,
        IpAddr,
        Ratelimit,
        SharePeersMaxLen,
    }
    fn inner(
        node: &mut Node,
        message: &gossipsub::Message,
        propagation_source: PeerId,
    ) -> Result<(), Error> {
        let source = message.source.ok_or(Error::MessageSource)?;
        let vec_ip_addr = node.p2p.vec_ip_addr(&[source, propagation_source]);
        if vec_ip_addr.is_empty() {
            return Err(Error::IpAddr);
        }
        let endpoint = match message.topic.as_str() {
            "block" => Endpoint::GossipsubMessageBlock,
            "transaction" => Endpoint::GossipsubMessageTransaction,
            "stake" => Endpoint::GossipsubMessageStake,
            "peers" => Endpoint::GossipsubMessagePeers,
            _ => unreachable!(),
        };
        for ip_addr in vec_ip_addr {
            if node.p2p.ratelimit.counter.add(ip_addr, &endpoint) {
                return Err(Error::Ratelimit);
            }
        }
        match endpoint {
            Endpoint::GossipsubMessageBlock => {
                let block: Block = bincode::deserialize(&message.data).map_err(Error::Bincode)?;
                node.blockchain
                    .pending_blocks_push(&node.db, block, node.args.time_delta, node.args.trust)
                    .map_err(Error::Blockchain)?;
                node.blockchain.save_blocks(&node.db, node.args.trust);
            }
            Endpoint::GossipsubMessageTransaction => {
                let transaction: Transaction =
                    bincode::deserialize(&message.data).map_err(Error::Bincode)?;
                node.blockchain
                    .pending_transactions_push(transaction, node.args.time_delta)
                    .map_err(Error::Blockchain)?;
            }
            Endpoint::GossipsubMessageStake => {
                let stake: Stake = bincode::deserialize(&message.data).map_err(Error::Bincode)?;
                node.blockchain
                    .pending_stakes_push(stake, node.args.time_delta)
                    .map_err(Error::Blockchain)?;
            }
            Endpoint::GossipsubMessagePeers => {
                let vec =
                    bincode::deserialize::<Vec<IpAddr>>(&message.data).map_err(Error::Bincode)?;
                if vec.len() > SHARE_PEERS_MAX_LEN {
                    return Err(Error::SharePeersMaxLen);
                }
                for ip_addr in vec {
                    node.p2p.connections_unknown.insert(ip_addr);
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    match match inner(node, &message, propagation_source) {
        Ok(()) => {
            debug!("Gossipsub message processed");
            node.p2p
                .swarm
                .behaviour_mut()
                .gossipsub
                .report_message_validation_result(
                    &message_id,
                    &propagation_source,
                    MessageAcceptance::Accept,
                )
        }
        Err(Error::Blockchain(blockchain::Error::BlockPending))
        | Err(Error::Blockchain(blockchain::Error::BlockHashInTree))
        | Err(Error::Blockchain(blockchain::Error::BlockPreviousHashNotInTree)) => node
            .p2p
            .swarm
            .behaviour_mut()
            .gossipsub
            .report_message_validation_result(
                &message_id,
                &propagation_source,
                MessageAcceptance::Ignore,
            ),
        Err(e) => {
            error!(?e);
            node.p2p
                .swarm
                .behaviour_mut()
                .gossipsub
                .report_message_validation_result(
                    &message_id,
                    &propagation_source,
                    MessageAcceptance::Reject,
                )
        }
    } {
        Ok(cache) => debug!(cache, "Message validation result reported"),
        Err(e) => error!(?e),
    }
}
#[instrument(skip_all, level = "trace")]
fn sync_request(
    node: &mut Node,
    peer_id: PeerId,
    request: Request,
    channel: ResponseChannel<Response>,
) {
    let ip_addr = match node.p2p.connections.get(&peer_id) {
        Some(x) => *x,
        None => {
            warn!("Peer {} not found in connections", peer_id);
            return;
        }
    };
    if node.p2p.ratelimit.counter.add(ip_addr, &Endpoint::Request) {
        return;
    }
    #[derive(Debug)]
    enum Error {
        Bincode(bincode::Error),
        Blockchain(blockchain::Error),
        Response(Response),
    }
    fn inner(
        node: &mut Node,
        request: Request,
        channel: ResponseChannel<Response>,
    ) -> Result<(), Error> {
        let height: usize = bincode::deserialize(&request.0).map_err(Error::Bincode)?;
        let mut size = 0;
        let mut vec = vec![];
        loop {
            let index = height + vec.len();
            let res = node.blockchain.sync_block(&node.db, index);
            if let Err(blockchain::Error::SyncBlock) = res {
                break;
            }
            let block = res.map_err(Error::Blockchain)?;
            size += bincode::serialize(&block).map_err(Error::Bincode)?.len();
            if size > MAX_TRANSMIT_SIZE {
                break;
            }
            vec.push(block);
        }
        let vec = bincode::serialize(&vec).map_err(Error::Bincode)?;
        node.p2p
            .swarm
            .behaviour_mut()
            .request_response
            .send_response(channel, Response(vec))
            .map_err(Error::Response)?;
        Ok(())
    }
    match inner(node, request, channel) {
        Ok(()) => debug!("Sync request processed"),
        Err(e) => {
            error!(?e);
            node.p2p
                .ratelimit
                .timeout
                .insert(ip_addr, Endpoint::Request);
        }
    }
}
#[instrument(skip_all, level = "trace")]
fn sync_response(node: &mut Node, peer_id: PeerId, response: Response) {
    let ip_addr = match node.p2p.connections.get(&peer_id) {
        Some(x) => *x,
        None => {
            warn!("Peer {} not found in connections", peer_id);
            return;
        }
    };
    if node.p2p.ratelimit.counter.add(ip_addr, &Endpoint::Response) {
        return;
    }
    #[derive(Debug)]
    enum Error {
        Bincode(bincode::Error),
        Blockchain(blockchain::Error),
    }
    fn inner(node: &mut Node, response: Response) -> Result<(), Error> {
        for block in bincode::deserialize::<Vec<Block>>(&response.0).map_err(Error::Bincode)? {
            node.blockchain
                .pending_blocks_push(&node.db, block, node.args.time_delta, node.args.trust)
                .map_err(Error::Blockchain)?;
            node.blockchain.save_blocks(&node.db, node.args.trust);
        }
        Ok(())
    }
    match inner(node, response) {
        Ok(()) => debug!("Sync response processed"),
        Err(e) => error!(?e, ?peer_id),
    }
}
