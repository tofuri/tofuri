use crate::behaviour::{FileRequest, FileResponse};
use crate::{multiaddr, node::Node};
use libp2p::{request_response::ResponseChannel, Multiaddr, PeerId};
use pea_block::BlockB;
use std::error::Error;
pub fn request_handler(node: &mut Node, peer_id: PeerId, request: FileRequest, channel: ResponseChannel<FileResponse>) -> Result<(), Box<dyn Error>> {
    Ok(())
}
pub fn response_handler(node: &mut Node, peer_id: PeerId, response: FileResponse) -> Result<(), Box<dyn Error>> {
    Ok(())
}
