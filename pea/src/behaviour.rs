use async_trait::async_trait;
use futures::prelude::*;
use libp2p::{
    autonat,
    core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName},
    gossipsub::{Gossipsub, GossipsubConfigBuilder, GossipsubEvent, MessageAuthenticity},
    identify, identity, mdns, ping,
    request_response::{ProtocolSupport, RequestResponse, RequestResponseCodec, RequestResponseEvent},
    swarm::NetworkBehaviour,
};
use pea_core::*;
use std::error::Error;
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
