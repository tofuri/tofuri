use crate::{node::Node, util};
use async_trait::async_trait;
use futures::prelude::*;
use libp2p::{
    autonat,
    core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName},
    gossipsub::{Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, MessageAuthenticity},
    identify, identity, mdns, ping,
    request_response::{ProtocolSupport, RequestResponse, RequestResponseCodec, RequestResponseEvent, ResponseChannel},
    swarm::NetworkBehaviour,
    Multiaddr, PeerId,
};
use pea_block::BlockB;
use pea_core::*;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::collections::HashMap;
use std::error::Error;
use std::net::IpAddr;
use tokio::io;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRequest(pub Vec<u8>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResponse(pub Vec<u8>);
#[derive(Debug, Clone)]
pub struct FileExchangeProtocol();
impl ProtocolName for FileExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        PROTOCOL_NAME.as_bytes()
    }
}
#[derive(Clone)]
pub struct FileExchangeCodec();
#[async_trait]
impl RequestResponseCodec for FileExchangeCodec {
    type Protocol = FileExchangeProtocol;
    type Request = FileRequest;
    type Response = FileResponse;
    async fn read_request<T: AsyncRead + Unpin + Send>(&mut self, _: &FileExchangeProtocol, io: &mut T) -> io::Result<Self::Request> {
        Ok(FileRequest(read_length_prefixed(io, 32).await?))
    }
    async fn read_response<T: AsyncRead + Unpin + Send>(&mut self, _: &FileExchangeProtocol, io: &mut T) -> io::Result<Self::Response> {
        Ok(FileResponse(read_length_prefixed(io, BLOCK_SIZE_LIMIT * SYNC_BLOCKS_PER_TICK).await?))
    }
    async fn write_request<T: AsyncWrite + Unpin + Send>(&mut self, _: &FileExchangeProtocol, io: &mut T, FileRequest(vec): FileRequest) -> io::Result<()> {
        write_length_prefixed(io, vec).await?;
        io.close().await?;
        Ok(())
    }
    async fn write_response<T: AsyncWrite + Unpin + Send>(&mut self, _: &FileExchangeProtocol, io: &mut T, FileResponse(vec): FileResponse) -> io::Result<()> {
        write_length_prefixed(io, vec).await?;
        io.close().await?;
        Ok(())
    }
}
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct Behaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub gossipsub: Gossipsub,
    pub autonat: autonat::Behaviour,
    pub request_response: RequestResponse<FileExchangeCodec>,
}
impl Behaviour {
    pub async fn new(local_key: identity::Keypair) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default())?,
            identify: identify::Behaviour::new(identify::Config::new(PROTOCOL_VERSION.to_string(), local_key.public())),
            gossipsub: Gossipsub::new(
                MessageAuthenticity::Signed(local_key.clone()),
                GossipsubConfigBuilder::default().max_transmit_size(BLOCK_SIZE_LIMIT).build()?,
            )?,
            autonat: autonat::Behaviour::new(local_key.public().to_peer_id(), autonat::Config::default()),
            request_response: RequestResponse::new(
                FileExchangeCodec(),
                std::iter::once((FileExchangeProtocol(), ProtocolSupport::Full)),
                Default::default(),
            ),
        })
    }
}
#[derive(Debug)]
pub enum OutEvent {
    Gossipsub(GossipsubEvent),
    Mdns(mdns::Event),
    Ping(ping::Event),
    Identify(identify::Event),
    Autonat(autonat::Event),
    RequestResponse(RequestResponseEvent<FileRequest, FileResponse>),
}
impl From<mdns::Event> for OutEvent {
    fn from(v: mdns::Event) -> Self {
        Self::Mdns(v)
    }
}
impl From<GossipsubEvent> for OutEvent {
    fn from(v: GossipsubEvent) -> Self {
        Self::Gossipsub(v)
    }
}
impl From<identify::Event> for OutEvent {
    fn from(v: identify::Event) -> Self {
        Self::Identify(v)
    }
}
impl From<autonat::Event> for OutEvent {
    fn from(v: autonat::Event) -> Self {
        Self::Autonat(v)
    }
}
impl From<RequestResponseEvent<FileRequest, FileResponse>> for OutEvent {
    fn from(v: RequestResponseEvent<FileRequest, FileResponse>) -> Self {
        Self::RequestResponse(v)
    }
}
pub mod request_response {
    use super::*;
    pub fn request_handler(node: &mut Node, peer_id: PeerId, request: FileRequest, channel: ResponseChannel<FileResponse>) -> Result<(), Box<dyn Error>> {
        println!("{:?}", request);
        Ok(())
    }
    pub fn response_handler(node: &mut Node, peer_id: PeerId, response: FileResponse) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
pub mod multiaddr {
    use super::*;
    use libp2p::multiaddr::Protocol;
    pub fn filter_ip(multiaddr: &Multiaddr) -> Option<Multiaddr> {
        let components = multiaddr.iter().collect::<Vec<_>>();
        let mut multiaddr: Multiaddr = "".parse().unwrap();
        match components.get(0) {
            Some(Protocol::Ip4(ip)) => multiaddr.push(Protocol::Ip4(*ip)),
            Some(Protocol::Ip6(ip)) => multiaddr.push(Protocol::Ip6(*ip)),
            _ => return None,
        };
        Some(multiaddr)
    }
    pub fn filter_ip_port(multiaddr: &Multiaddr) -> Option<Multiaddr> {
        let components = multiaddr.iter().collect::<Vec<_>>();
        let mut multiaddr: Multiaddr = "".parse().unwrap();
        match components.get(0) {
            Some(Protocol::Ip4(ip)) => multiaddr.push(Protocol::Ip4(*ip)),
            Some(Protocol::Ip6(ip)) => multiaddr.push(Protocol::Ip6(*ip)),
            _ => return None,
        };
        match components.get(1) {
            Some(Protocol::Tcp(port)) => {
                if port == &9333_u16 {
                    return Some(multiaddr);
                }
                multiaddr.push(Protocol::Tcp(*port))
            }
            _ => return Some(multiaddr),
        };
        Some(multiaddr)
    }
    pub fn has_port(multiaddr: &Multiaddr) -> bool {
        let components = multiaddr.iter().collect::<Vec<_>>();
        matches!(components.get(1), Some(Protocol::Tcp(_)))
    }
    pub fn addr(multiaddr: &Multiaddr) -> Option<IpAddr> {
        match multiaddr.iter().collect::<Vec<_>>().first() {
            Some(Protocol::Ip4(ip)) => Some(IpAddr::V4(*ip)),
            Some(Protocol::Ip6(ip)) => Some(IpAddr::V6(*ip)),
            _ => None,
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_filter_ip() {
            assert_eq!(filter_ip(&"".parse::<Multiaddr>().unwrap()), None);
            assert_eq!(filter_ip(&"/tcp/9333".parse::<Multiaddr>().unwrap()), None);
            assert_eq!(
                filter_ip(&"/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()).unwrap(),
                "/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()
            );
        }
        #[test]
        fn test_filter_ip_port() {
            assert_eq!(filter_ip_port(&"".parse::<Multiaddr>().unwrap()), None);
            assert_eq!(filter_ip_port(&"/tcp/9333".parse::<Multiaddr>().unwrap()), None);
            assert_eq!(
                filter_ip_port(&"/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()).unwrap(),
                "/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()
            );
            assert_eq!(
                filter_ip_port(&"/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()).unwrap(),
                "/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()
            );
            assert_eq!(
                filter_ip_port(&"/ip4/0.0.0.0/tcp/9334".parse::<Multiaddr>().unwrap()).unwrap(),
                "/ip4/0.0.0.0/tcp/9334".parse::<Multiaddr>().unwrap()
            );
        }
        #[test]
        fn test_has_port() {
            assert_eq!(has_port(&"".parse::<Multiaddr>().unwrap()), false);
            assert_eq!(has_port(&"/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()), false);
            assert_eq!(has_port(&"/tcp/9333".parse::<Multiaddr>().unwrap()), false);
            assert_eq!(has_port(&"/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()), true);
        }
    }
}
pub mod gossipsub {
    use super::*;
    pub fn handler(node: &mut Node, message: GossipsubMessage, propagation_source: PeerId) -> Result<(), Box<dyn Error>> {
        let (multiaddr, _) = node.p2p_connections.iter().find(|x| x.1 == &propagation_source).unwrap();
        let addr = multiaddr::addr(multiaddr).expect("multiaddr to include ip");
        match message.topic.as_str() {
            "block" => {
                ratelimit(node, addr, propagation_source, Topic::Block)?;
                let block_b: BlockB = bincode::deserialize(&message.data)?;
                node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
            }
            "blocks" => {
                ratelimit(node, addr, propagation_source, Topic::Blocks)?;
                for block_b in bincode::deserialize::<Vec<BlockB>>(&message.data)? {
                    ratelimit(node, addr, propagation_source, Topic::Block)?;
                    node.blockchain.pending_blocks_push(block_b, util::timestamp())?;
                }
            }
            "transaction" => {
                ratelimit(node, addr, propagation_source, Topic::Transaction)?;
                let transaction_b: TransactionB = bincode::deserialize(&message.data)?;
                node.blockchain.pending_transactions_push(transaction_b, util::timestamp())?;
            }
            "stake" => {
                ratelimit(node, addr, propagation_source, Topic::Stake)?;
                let stake_b: StakeB = bincode::deserialize(&message.data)?;
                node.blockchain.pending_stakes_push(stake_b, util::timestamp())?;
            }
            "multiaddr" => {
                ratelimit(node, addr, propagation_source, Topic::Multiaddr)?;
                for multiaddr in bincode::deserialize::<Vec<Multiaddr>>(&message.data)? {
                    if let Some(multiaddr) = multiaddr::filter_ip_port(&multiaddr) {
                        node.p2p_unknown.insert(multiaddr);
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
}
pub fn ratelimit(node: &mut Node, addr: IpAddr, propagation_source: PeerId, topic: Topic) -> Result<(), Box<dyn Error>> {
    if node.p2p_ratelimit.add(addr, topic) {
        let _ = node.p2p_swarm.disconnect_peer_id(propagation_source);
        return Err("ratelimited".into());
    }
    Ok(())
}
pub enum Topic {
    Block,
    Transaction,
    Stake,
    Multiaddr,
    Blocks,
}
#[derive(Debug, Default)]
pub struct Ratelimit {
    map: HashMap<IpAddr, ([usize; 5], Option<u32>)>,
}
impl Ratelimit {
    pub fn get(&self, addr: &IpAddr) -> ([usize; 5], Option<u32>) {
        match self.map.get(addr) {
            Some(x) => *x,
            None => ([0; 5], None),
        }
    }
    pub fn is_ratelimited(&self, b: &Option<u32>) -> bool {
        if let Some(timestamp) = b {
            if timestamp + RATELIMIT_DURATION > util::timestamp() {
                return true;
            }
        }
        false
    }
    pub fn add(&mut self, addr: IpAddr, topic: Topic) -> bool {
        let mut value = self.get(&addr);
        let a = &mut value.0;
        let b = &mut value.1;
        if self.is_ratelimited(b) {
            return true;
        }
        let ratelimited = match topic {
            Topic::Block => {
                a[0] += 1;
                a[0] > RATELIMIT_TOPIC_BLOCK
            }
            Topic::Blocks => {
                a[1] += 1;
                a[1] > RATELIMIT_TOPIC_BLOCKS
            }
            Topic::Transaction => {
                a[2] += 1;
                a[2] > RATELIMIT_TOPIC_TRANSACTION
            }
            Topic::Stake => {
                a[3] += 1;
                a[3] > RATELIMIT_TOPIC_STAKE
            }
            Topic::Multiaddr => {
                a[4] += 1;
                a[4] > RATELIMIT_TOPIC_MULTIADDR
            }
        };
        if ratelimited {
            *b = Some(util::timestamp());
        }
        self.map.insert(addr, value);
        ratelimited
    }
    pub fn reset(&mut self) {
        for value in self.map.values_mut() {
            let a = &mut value.0;
            a[0] = a[0].saturating_sub(RATELIMIT_TOPIC_BLOCK);
            a[1] = a[1].saturating_sub(RATELIMIT_TOPIC_BLOCKS);
            a[2] = a[2].saturating_sub(RATELIMIT_TOPIC_TRANSACTION);
            a[3] = a[3].saturating_sub(RATELIMIT_TOPIC_STAKE);
            a[4] = a[4].saturating_sub(RATELIMIT_TOPIC_MULTIADDR);
        }
    }
}
