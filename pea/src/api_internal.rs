use crate::Node;
use colored::*;
use libp2p::Multiaddr;
use log::error;
use log::info;
use pea_api_internal_core::Data;
use pea_api_internal_core::Data::Balance;
use pea_api_internal_core::Data::BlockByHash;
use pea_api_internal_core::Data::BlockLatest;
use pea_api_internal_core::Data::HashByHeight;
use pea_api_internal_core::Data::Height;
use pea_api_internal_core::Data::HeightByHash;
use pea_api_internal_core::Data::Peer;
use pea_api_internal_core::Data::Peers;
use pea_api_internal_core::Data::Stake;
use pea_api_internal_core::Data::StakeByHash;
use pea_api_internal_core::Data::Staked;
use pea_api_internal_core::Data::Transaction;
use pea_api_internal_core::Data::TransactionByHash;
use pea_api_internal_core::Request;
use pea_block::BlockA;
use pea_core::*;
use pea_db as db;
use pea_p2p::multiaddr;
use pea_stake::StakeA;
use pea_stake::StakeB;
use pea_transaction::TransactionA;
use pea_transaction::TransactionB;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;
pub async fn accept(node: &mut Node, res: Result<(TcpStream, SocketAddr), io::Error>) {
    if let Err(err) = &res {
        error!("{} {}", "API".cyan(), err);
    }
    let (stream, socket_addr) = res.unwrap();
    match request(node, stream).await {
        Ok((bytes, data)) => info!(
            "{} {} {} {:?}",
            "API".cyan(),
            socket_addr.to_string().magenta(),
            bytes.to_string().yellow(),
            data
        ),
        Err(err) => error!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), err),
    };
}
async fn request(node: &mut Node, mut stream: TcpStream) -> Result<(usize, Data), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let bytes = timeout(Duration::from_millis(1), stream.read(&mut buffer)).await??;
    let request: Request = bincode::deserialize(&buffer)?;
    stream
        .write_all(&match request.data {
            Balance => bincode::serialize(&balance(node, &request.vec)?),
            Staked => bincode::serialize(&staked(node, &request.vec)?),
            Height => bincode::serialize(&height(node)?),
            HeightByHash => bincode::serialize(&height_by_hash(node, &request.vec)?),
            BlockLatest => bincode::serialize(block_latest(node)?),
            HashByHeight => bincode::serialize(&hash_by_height(node, &request.vec)?),
            BlockByHash => bincode::serialize(&block_by_hash(node, &request.vec)?),
            TransactionByHash => bincode::serialize(&transaction_by_hash(node, &request.vec)?),
            StakeByHash => bincode::serialize(&stake_by_hash(node, &request.vec)?),
            Peers => bincode::serialize(&peers(node)?),
            Peer => bincode::serialize(&peer(node, &request.vec)?),
            Transaction => bincode::serialize(&transaction(node, &request.vec)?),
            Stake => bincode::serialize(&stake(node, &request.vec)?),
        }?)
        .await?;
    stream.flush().await?;
    Ok((bytes, request.data))
}
fn balance(node: &mut Node, bytes: &[u8]) -> Result<u128, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    Ok(node.blockchain.states.dynamic.balance(&address_bytes))
}
fn staked(node: &mut Node, bytes: &[u8]) -> Result<u128, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    Ok(node.blockchain.states.dynamic.staked(&address_bytes))
}
fn height(node: &mut Node) -> Result<usize, Box<dyn Error>> {
    Ok(node.blockchain.height())
}
fn height_by_hash(node: &mut Node, bytes: &[u8]) -> Result<usize, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(&bytes)?;
    let block_c = db::block::get_c(&node.db, &hash)?;
    Ok(node.blockchain.tree.height(&block_c.previous_hash))
}
fn block_latest(node: &mut Node) -> Result<&BlockA, Box<dyn Error>> {
    Ok(&node.blockchain.states.dynamic.latest_block)
}
fn hash_by_height(node: &mut Node, bytes: &[u8]) -> Result<Hash, Box<dyn Error>> {
    let height: usize = bincode::deserialize(bytes)?;
    let states = &node.blockchain.states;
    let hashes_trusted = &states.trusted.hashes;
    let hashes_dynamic = &states.dynamic.hashes;
    if height >= hashes_trusted.len() + hashes_dynamic.len() {
        return Err("GET HEIGHT_HASH".into());
    }
    let hash = if height < hashes_trusted.len() {
        hashes_trusted[height]
    } else {
        hashes_dynamic[height - hashes_trusted.len()]
    };
    Ok(hash)
}
fn block_by_hash(node: &mut Node, bytes: &[u8]) -> Result<BlockA, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(bytes)?;
    Ok(db::block::get_a(&node.db, &hash)?)
}
fn transaction_by_hash(node: &mut Node, bytes: &[u8]) -> Result<TransactionA, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(bytes)?;
    Ok(db::transaction::get_a(&node.db, &hash)?)
}
fn stake_by_hash(node: &mut Node, bytes: &[u8]) -> Result<StakeA, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(bytes)?;
    Ok(db::stake::get_a(&node.db, &hash)?)
}
fn peers(node: &mut Node) -> Result<Vec<&Multiaddr>, Box<dyn Error>> {
    Ok(node.p2p.connections.keys().collect())
}
fn peer(node: &mut Node, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let multiaddr: Multiaddr = bincode::deserialize(bytes)?;
    let multiaddr = multiaddr::ip_port(&multiaddr).ok_or("multiaddr filter_ip_port")?;
    node.p2p.unknown.insert(multiaddr);
    Ok(())
}
fn transaction(node: &mut Node, bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let transaction_b: TransactionB = bincode::deserialize(bytes)?;
    let data = bincode::serialize(&transaction_b).unwrap();
    let status = match node
        .blockchain
        .pending_transactions_push(&node.db, transaction_b, pea_util::timestamp(), node.args.time_delta)
    {
        Ok(()) => {
            if node.p2p.gossipsub_has_mesh_peers("transaction") {
                if let Err(err) = node.p2p.gossipsub_publish("transaction", data) {
                    error!("{}", err);
                }
            }
            "success".to_string()
        }
        Err(err) => {
            error!("{}", err);
            err.to_string()
        }
    };
    Ok(status)
}
fn stake(node: &mut Node, bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let stake_b: StakeB = bincode::deserialize(bytes)?;
    let data = bincode::serialize(&stake_b).unwrap();
    let status = match node
        .blockchain
        .pending_stakes_push(&node.db, stake_b, pea_util::timestamp(), node.args.time_delta)
    {
        Ok(()) => {
            if node.p2p.gossipsub_has_mesh_peers("stake") {
                if let Err(err) = node.p2p.gossipsub_publish("stake", data) {
                    error!("{}", err);
                }
            }
            "success".to_string()
        }
        Err(err) => {
            error!("{}", err);
            err.to_string()
        }
    };
    Ok(status)
}
