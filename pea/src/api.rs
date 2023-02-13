use crate::Node;
use colored::*;
use libp2p::Multiaddr;
use log::error;
use log::info;
use pea_api as api;
use pea_api_core::internal::Data::Args;
use pea_api_core::internal::Data::Balance;
use pea_api_core::internal::Data::BlockByHash;
use pea_api_core::internal::Data::BlockLatest;
use pea_api_core::internal::Data::Dynamic;
use pea_api_core::internal::Data::HashByHeight;
use pea_api_core::internal::Data::Height;
use pea_api_core::internal::Data::HeightByHash;
use pea_api_core::internal::Data::Info;
use pea_api_core::internal::Data::Peer;
use pea_api_core::internal::Data::Peers;
use pea_api_core::internal::Data::Stake;
use pea_api_core::internal::Data::StakeByHash;
use pea_api_core::internal::Data::Staked;
use pea_api_core::internal::Data::Sync;
use pea_api_core::internal::Data::Transaction;
use pea_api_core::internal::Data::TransactionByHash;
use pea_api_core::internal::Data::Trusted;
use pea_api_core::internal::Request;
use pea_core::*;
use pea_db as db;
use pea_p2p::multiaddr;
use pea_stake::StakeB;
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
    match res {
        Ok((stream, socket_addr)) => match request(node, stream).await {
            Ok((bytes, first)) => info!(
                "{} {} {} {}",
                "API".cyan(),
                socket_addr.to_string().magenta(),
                bytes.to_string().yellow(),
                first
            ),
            Err(err) => error!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), err),
        },
        Err(err) => error!("{} {}", "API".cyan(), err),
    }
}
async fn request(node: &mut Node, mut stream: TcpStream) -> Result<(usize, String), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let bytes = timeout(Duration::from_millis(1), stream.read(&mut buffer)).await??;
    let request: Request = bincode::deserialize(&buffer)?;
    stream
        .write_all(&match request.data {
            Info => info(node),
            Sync => sync(node),
            Dynamic => dynamic(node),
            Trusted => trusted(node),
            Args => args(node),
            Balance => balance(node, &request.vec),
            Staked => staked(node, &request.vec),
            Height => height(node),
            HeightByHash => height_by_hash(node, &request.vec),
            BlockLatest => block_latest(node),
            HashByHeight => hash_by_height(node, &request.vec),
            BlockByHash => block_by_hash(node, &request.vec),
            TransactionByHash => transaction_by_hash(node, &request.vec),
            StakeByHash => stake_by_hash(node, &request.vec),
            Peers => peers(node),
            Peer => peer(node, &request.vec),
            Transaction => transaction(node, &request.vec),
            Stake => stake(node, &request.vec),
        }?)
        .await?;
    stream.flush().await?;
    Ok((bytes, "".to_string()))
}
fn info(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(bincode::serialize(&api::Info::from(
        &node.key,
        node.ticks,
        node.args.tps,
        &node.blockchain,
        node.lag,
    ))?)
}
fn sync(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(bincode::serialize(&api::Sync::from(&node.blockchain))?)
}
fn dynamic(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    let dynamic = &node.blockchain.states.dynamic;
    Ok(bincode::serialize(&api::Dynamic::from(&dynamic))?)
}
fn trusted(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    let trusted = &node.blockchain.states.trusted;
    Ok(bincode::serialize(&api::Trusted::from(&trusted))?)
}
fn args(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(bincode::serialize(&node.args)?)
}
fn balance(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    let balance = node.blockchain.states.dynamic.balance(&address_bytes);
    Ok(bincode::serialize(&pea_int::to_string(balance))?)
}
fn staked(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    let balance = node.blockchain.states.dynamic.staked(&address_bytes);
    Ok(bincode::serialize(&pea_int::to_string(balance))?)
}
fn height(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    let height = node.blockchain.height();
    Ok(bincode::serialize(&height)?)
}
fn height_by_hash(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let hash: Vec<u8> = bincode::deserialize(bytes)?;
    let block_c = db::block::get_c(&node.db, &hash)?;
    let height = node.blockchain.tree.height(&block_c.previous_hash);
    Ok(bincode::serialize(&height)?)
}
fn block_latest(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    let block_a = &node.blockchain.states.dynamic.latest_block;
    Ok(bincode::serialize(&api::Block::from(&block_a))?)
}
fn hash_by_height(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
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
    Ok(bincode::serialize(&hex::encode(hash))?)
}
fn block_by_hash(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let hash: Vec<u8> = bincode::deserialize(bytes)?;
    let block_a = db::block::get_a(&node.db, &hash)?;
    Ok(bincode::serialize(&api::Block::from(&block_a))?)
}
fn transaction_by_hash(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let hash: Vec<u8> = bincode::deserialize(bytes)?;
    let transaction_a = db::transaction::get_a(&node.db, &hash)?;
    Ok(bincode::serialize(&api::Transaction::from(&transaction_a))?)
}
fn stake_by_hash(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let hash: Vec<u8> = bincode::deserialize(bytes)?;
    let stake_a = db::stake::get_a(&node.db, &hash)?;
    Ok(bincode::serialize(&api::Stake::from(&stake_a))?)
}
fn peers(node: &mut Node) -> Result<Vec<u8>, Box<dyn Error>> {
    let peers: Vec<&Multiaddr> = node.p2p.connections.keys().collect();
    Ok(bincode::serialize(&peers)?)
}
fn peer(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let multiaddr: Multiaddr = bincode::deserialize(bytes)?;
    let multiaddr = multiaddr::ip_port(&multiaddr).ok_or("multiaddr filter_ip_port")?;
    let string = multiaddr.to_string();
    node.p2p.unknown.insert(multiaddr);
    Ok(bincode::serialize(&string)?)
}
fn transaction(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
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
    Ok(bincode::serialize(&status)?)
}
fn stake(node: &mut Node, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
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
    Ok(bincode::serialize(&status)?)
}
