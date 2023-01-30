use crate::node::Node;
use libp2p::gossipsub::GossipsubMessage;
use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use libp2p::PeerId;
use log::debug;
use pea_block::BlockB;
use pea_core::*;
use pea_p2p::behaviour::SyncRequest;
use pea_p2p::behaviour::SyncResponse;
use pea_p2p::ratelimit::Endpoint;
use pea_p2p::ratelimit::Ratelimit;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use pea_util;
use std::error::Error;
pub fn gossipsub_handler(node: &mut Node, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn Error>> {
    match message.topic.as_str() {
        "block" => {
            Ratelimit::ratelimit(&mut node.p2p, propagation_source, Endpoint::Block)?;
            let block_b: BlockB = bincode::deserialize(&message.data)?;
            node.blockchain.append_block(block_b, pea_util::timestamp())?;
        }
        "transaction" => {
            Ratelimit::ratelimit(&mut node.p2p, propagation_source, Endpoint::Transaction)?;
            let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_transactions_push(transaction_b, pea_util::timestamp())?;
        }
        "stake" => {
            Ratelimit::ratelimit(&mut node.p2p, propagation_source, Endpoint::Stake)?;
            let stake_b: StakeB = bincode::deserialize(&message.data)?;
            node.blockchain.pending_stakes_push(stake_b, pea_util::timestamp())?;
        }
        "multiaddr" => {
            Ratelimit::ratelimit(&mut node.p2p, propagation_source, Endpoint::Multiaddr)?;
            for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                if let Some(multiaddr) = pea_p2p::multiaddr::multiaddr_filter_ip_port(&multiaddr) {
                    node.p2p.unknown.insert(multiaddr);
                }
            }
        }
        _ => {}
    };
    Ok(())
}
pub fn request_handler(node: &mut Node, peer_id: PeerId, request: SyncRequest, channel: ResponseChannel<SyncResponse>) -> Result<(), Box<dyn Error>> {
    Ratelimit::ratelimit(&mut node.p2p, peer_id, Endpoint::SyncRequest)?;
    let height: usize = bincode::deserialize(&request.0)?;
    let mut vec = vec![];
    for i in 0..SYNC_BLOCKS_PER_TICK {
        match node.blockchain.sync_block(height + i) {
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
pub fn response_handler(node: &mut Node, peer_id: PeerId, response: SyncResponse) -> Result<(), Box<dyn Error>> {
    Ratelimit::ratelimit(&mut node.p2p, peer_id, Endpoint::SyncResponse)?;
    let timestamp = pea_util::timestamp();
    for block_b in bincode::deserialize::<Vec<BlockB>>(&response.0)? {
        if let Err(err) = node.blockchain.append_block(block_b, timestamp) {
            debug!("response_handler {}", err);
        }
    }
    Ok(())
}
