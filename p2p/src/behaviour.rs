use async_trait::async_trait;
use futures::prelude::*;
use libp2p::autonat;
use libp2p::core::upgrade::read_length_prefixed;
use libp2p::core::upgrade::write_length_prefixed;
use libp2p::core::upgrade::ProtocolName;
use libp2p::gossipsub::Gossipsub;
use libp2p::gossipsub::GossipsubConfigBuilder;
use libp2p::gossipsub::GossipsubEvent;
use libp2p::gossipsub::MessageAuthenticity;
use libp2p::identify;
use libp2p::identity;
use libp2p::mdns;
use libp2p::request_response::ProtocolSupport;
use libp2p::request_response::RequestResponse;
use libp2p::request_response::RequestResponseCodec;
use libp2p::request_response::RequestResponseEvent;
use libp2p::swarm::NetworkBehaviour;
use tofuri_core::*;
use tokio::io;
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
}
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct Behaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub gossipsub: Gossipsub,
    pub autonat: autonat::Behaviour,
    pub request_response: RequestResponse<SyncCodec>,
}
impl Behaviour {
    pub async fn new(local_key: identity::Keypair) -> Result<Self, Error> {
        Ok(Self {
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default()).map_err(Error::Io)?,
            identify: identify::Behaviour::new(identify::Config::new(PROTOCOL_VERSION.to_string(), local_key.public())),
            gossipsub: Gossipsub::new(
                MessageAuthenticity::Signed(local_key.clone()),
                GossipsubConfigBuilder::default()
                    .max_transmit_size(MAX_TRANSMIT_SIZE)
                    .validate_messages()
                    .build()
                    .unwrap(),
            )
            .unwrap(),
            autonat: autonat::Behaviour::new(local_key.public().to_peer_id(), autonat::Config::default()),
            request_response: RequestResponse::new(SyncCodec(), std::iter::once((SyncProtocol(), ProtocolSupport::Full)), Default::default()),
        })
    }
}
#[derive(Debug)]
pub enum OutEvent {
    Gossipsub(GossipsubEvent),
    Mdns(mdns::Event),
    Identify(identify::Event),
    Autonat(autonat::Event),
    RequestResponse(RequestResponseEvent<SyncRequest, SyncResponse>),
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
impl From<RequestResponseEvent<SyncRequest, SyncResponse>> for OutEvent {
    fn from(v: RequestResponseEvent<SyncRequest, SyncResponse>) -> Self {
        Self::RequestResponse(v)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRequest(pub Vec<u8>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResponse(pub Vec<u8>);
#[derive(Debug, Clone)]
pub struct SyncProtocol();
impl ProtocolName for SyncProtocol {
    fn protocol_name(&self) -> &[u8] {
        PROTOCOL_NAME.as_bytes()
    }
}
#[derive(Clone)]
pub struct SyncCodec();
#[async_trait]
impl RequestResponseCodec for SyncCodec {
    type Protocol = SyncProtocol;
    type Request = SyncRequest;
    type Response = SyncResponse;
    async fn read_request<T: AsyncRead + Unpin + Send>(&mut self, _: &SyncProtocol, io: &mut T) -> io::Result<Self::Request> {
        Ok(SyncRequest(read_length_prefixed(io, 8).await?))
    }
    async fn read_response<T: AsyncRead + Unpin + Send>(&mut self, _: &SyncProtocol, io: &mut T) -> io::Result<Self::Response> {
        Ok(SyncResponse(read_length_prefixed(io, MAX_TRANSMIT_SIZE).await?))
    }
    async fn write_request<T: AsyncWrite + Unpin + Send>(&mut self, _: &SyncProtocol, io: &mut T, SyncRequest(vec): SyncRequest) -> io::Result<()> {
        write_length_prefixed(io, vec).await?;
        io.close().await?;
        Ok(())
    }
    async fn write_response<T: AsyncWrite + Unpin + Send>(&mut self, _: &SyncProtocol, io: &mut T, SyncResponse(vec): SyncResponse) -> io::Result<()> {
        write_length_prefixed(io, vec).await?;
        io.close().await?;
        Ok(())
    }
}
