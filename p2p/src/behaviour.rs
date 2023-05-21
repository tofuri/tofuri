use async_trait::async_trait;
use futures::prelude::*;
use libp2p::autonat;
use libp2p::connection_limits;
use libp2p::connection_limits::ConnectionLimits;
use libp2p::core::upgrade::read_length_prefixed;
use libp2p::core::upgrade::write_length_prefixed;
use libp2p::core::upgrade::ProtocolName;
use libp2p::gossipsub;
use libp2p::gossipsub::MessageAuthenticity;
use libp2p::identify;
use libp2p::identity;
use libp2p::mdns;
use libp2p::request_response;
use libp2p::request_response::ProtocolSupport;
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
    pub gossipsub: gossipsub::Behaviour,
    pub autonat: autonat::Behaviour,
    pub request_response: request_response::Behaviour<Codec>,
    pub connection_limits: connection_limits::Behaviour,
}
impl Behaviour {
    pub async fn new(
        local_key: identity::Keypair,
        max_established: Option<u32>,
    ) -> Result<Behaviour, Error> {
        let local_public_key = local_key.public();
        let local_peer_id = local_public_key.to_peer_id();
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
            .map_err(Error::Io)?;
        let identify = identify::Behaviour::new(identify::Config::new(
            PROTOCOL_VERSION.to_string(),
            local_public_key,
        ));
        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key),
            gossipsub::ConfigBuilder::default()
                .max_transmit_size(MAX_TRANSMIT_SIZE)
                .validate_messages()
                .build()
                .unwrap(),
        )
        .unwrap();
        let autonat = autonat::Behaviour::new(local_peer_id, autonat::Config::default());
        let request_response = request_response::Behaviour::new(
            Codec(),
            std::iter::once((Protocol(), ProtocolSupport::Full)),
            Default::default(),
        );
        let connection_limits = {
            let mut connection_limits = ConnectionLimits::default();
            connection_limits = connection_limits.with_max_established_per_peer(Some(1));
            connection_limits = connection_limits.with_max_established(max_established);
            connection_limits::Behaviour::new(connection_limits)
        };
        let behaviour = Behaviour {
            mdns,
            identify,
            gossipsub,
            autonat,
            request_response,
            connection_limits,
        };
        Ok(behaviour)
    }
}
#[derive(Debug)]
pub enum OutEvent {
    Gossipsub(gossipsub::Event),
    Mdns(mdns::Event),
    Identify(identify::Event),
    Autonat(autonat::Event),
    RequestResponse(request_response::Event<Request, Response>),
    Void(void::Void),
}
impl From<mdns::Event> for OutEvent {
    fn from(v: mdns::Event) -> OutEvent {
        OutEvent::Mdns(v)
    }
}
impl From<gossipsub::Event> for OutEvent {
    fn from(v: gossipsub::Event) -> OutEvent {
        OutEvent::Gossipsub(v)
    }
}
impl From<identify::Event> for OutEvent {
    fn from(v: identify::Event) -> OutEvent {
        OutEvent::Identify(v)
    }
}
impl From<autonat::Event> for OutEvent {
    fn from(v: autonat::Event) -> OutEvent {
        OutEvent::Autonat(v)
    }
}
impl From<request_response::Event<Request, Response>> for OutEvent {
    fn from(v: request_response::Event<Request, Response>) -> OutEvent {
        OutEvent::RequestResponse(v)
    }
}
impl From<void::Void> for OutEvent {
    fn from(v: void::Void) -> OutEvent {
        OutEvent::Void(v)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request(pub Vec<u8>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response(pub Vec<u8>);
#[derive(Debug, Clone)]
pub struct Protocol();
impl ProtocolName for Protocol {
    fn protocol_name(&self) -> &[u8] {
        PROTOCOL_NAME.as_bytes()
    }
}
#[derive(Clone)]
pub struct Codec();
#[async_trait]
impl request_response::Codec for Codec {
    type Protocol = Protocol;
    type Request = Request;
    type Response = Response;
    async fn read_request<T: AsyncRead + Unpin + Send>(
        &mut self,
        _: &Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request> {
        let vec = read_length_prefixed(io, 8).await?;
        let request = Request(vec);
        Ok(request)
    }
    async fn read_response<T: AsyncRead + Unpin + Send>(
        &mut self,
        _: &Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response> {
        let vec = read_length_prefixed(io, MAX_TRANSMIT_SIZE).await?;
        let response = Response(vec);
        Ok(response)
    }
    async fn write_request<T: AsyncWrite + Unpin + Send>(
        &mut self,
        _: &Protocol,
        io: &mut T,
        Request(vec): Request,
    ) -> io::Result<()> {
        write_length_prefixed(io, vec).await?;
        io.close().await?;
        Ok(())
    }
    async fn write_response<T: AsyncWrite + Unpin + Send>(
        &mut self,
        _: &Protocol,
        io: &mut T,
        Response(vec): Response,
    ) -> io::Result<()> {
        write_length_prefixed(io, vec).await?;
        io.close().await?;
        Ok(())
    }
}
