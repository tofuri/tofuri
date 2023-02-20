use crate::Node;
use colored::*;
use libp2p::Multiaddr;
use log::error;
use log::info;
use pea_api_internal_core::Data;
use pea_api_internal_core::Data::Address;
use pea_api_internal_core::Data::Balance;
use pea_api_internal_core::Data::BlockByHash;
use pea_api_internal_core::Data::BlockLatest;
use pea_api_internal_core::Data::CargoPkgName;
use pea_api_internal_core::Data::CargoPkgRepository;
use pea_api_internal_core::Data::CargoPkgVersion;
use pea_api_internal_core::Data::DynamicHashes;
use pea_api_internal_core::Data::DynamicLatestHashes;
use pea_api_internal_core::Data::DynamicStakers;
use pea_api_internal_core::Data::GitHash;
use pea_api_internal_core::Data::HashByHeight;
use pea_api_internal_core::Data::Height;
use pea_api_internal_core::Data::HeightByHash;
use pea_api_internal_core::Data::Lag;
use pea_api_internal_core::Data::Peer;
use pea_api_internal_core::Data::Peers;
use pea_api_internal_core::Data::RandomQueue;
use pea_api_internal_core::Data::Stake;
use pea_api_internal_core::Data::StakeByHash;
use pea_api_internal_core::Data::Staked;
use pea_api_internal_core::Data::Sync;
use pea_api_internal_core::Data::Ticks;
use pea_api_internal_core::Data::Time;
use pea_api_internal_core::Data::Transaction;
use pea_api_internal_core::Data::TransactionByHash;
use pea_api_internal_core::Data::TreeSize;
use pea_api_internal_core::Data::TrustedHashes;
use pea_api_internal_core::Data::TrustedLatestHashes;
use pea_api_internal_core::Data::TrustedStakers;
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
use tokio::time::Instant;
pub async fn accept(node: &mut Node, res: Result<(TcpStream, SocketAddr), io::Error>) -> Instant {
    let instant = Instant::now();
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
    instant
}
async fn request(node: &mut Node, mut stream: TcpStream) -> Result<(usize, Data), Box<dyn Error>> {
    let mut buf = [0; 1024];
    let bytes = timeout(Duration::from_millis(1), stream.read(&mut buf)).await??;
    let slice = &buf[..bytes];
    let request: Request = bincode::deserialize(slice)?;
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
            CargoPkgName => bincode::serialize(cargo_pkg_name()),
            CargoPkgVersion => bincode::serialize(cargo_pkg_version()),
            CargoPkgRepository => bincode::serialize(cargo_pkg_repository()),
            GitHash => bincode::serialize(git_hash()),
            Address => bincode::serialize(&address(node)),
            Ticks => bincode::serialize(ticks(node)),
            Lag => bincode::serialize(lag(node)),
            Time => bincode::serialize(&time()),
            TreeSize => bincode::serialize(&tree_size(node)),
            Sync => bincode::serialize(sync(node)),
            RandomQueue => bincode::serialize(&random_queue(node)),
            DynamicHashes => bincode::serialize(&dynamic_hashes(node)),
            DynamicLatestHashes => bincode::serialize(&dynamic_latest_hashes(node)),
            DynamicStakers => bincode::serialize(&dynamic_stakers(node)),
            TrustedHashes => bincode::serialize(&trusted_hashes(node)),
            TrustedLatestHashes => bincode::serialize(&trusted_latest_hashes(node)),
            TrustedStakers => bincode::serialize(&trusted_stakers(node)),
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
    let status = match node.blockchain.pending_transactions_push(transaction_b, node.args.time_delta) {
        Ok(()) => {
            if let Err(err) = node.p2p.gossipsub_publish("transaction", data) {
                error!("{}", err);
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
    let status = match node.blockchain.pending_stakes_push(stake_b, node.args.time_delta) {
        Ok(()) => {
            if let Err(err) = node.p2p.gossipsub_publish("stake", data) {
                error!("{}", err);
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
fn cargo_pkg_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}
fn cargo_pkg_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
fn cargo_pkg_repository() -> &'static str {
    env!("CARGO_PKG_REPOSITORY")
}
fn git_hash() -> &'static str {
    env!("GIT_HASH")
}
fn address(node: &mut Node) -> AddressBytes {
    node.key.address_bytes()
}
fn ticks(node: &mut Node) -> &usize {
    &node.ticks
}
fn lag(node: &mut Node) -> &f64 {
    &node.lag
}
fn time() -> i64 {
    chrono::offset::Utc::now().timestamp_millis()
}
fn tree_size(node: &mut Node) -> usize {
    node.blockchain.tree.size()
}
fn sync(node: &mut Node) -> &pea_blockchain::sync::Sync {
    &node.blockchain.sync
}
fn random_queue(node: &mut Node) -> Vec<AddressBytes> {
    node.blockchain.states.dynamic.stakers_n(8)
}
fn dynamic_hashes(node: &mut Node) -> usize {
    node.blockchain.states.dynamic.hashes.len()
}
fn dynamic_latest_hashes(node: &mut Node) -> Vec<&Hash> {
    node.blockchain.states.dynamic.hashes.iter().rev().take(16).collect()
}
fn dynamic_stakers(node: &mut Node) -> usize {
    node.blockchain.states.dynamic.stakers.len()
}
fn trusted_hashes(node: &mut Node) -> usize {
    node.blockchain.states.trusted.hashes.len()
}
fn trusted_latest_hashes(node: &mut Node) -> Vec<&Hash> {
    node.blockchain.states.trusted.hashes.iter().rev().take(16).collect()
}
fn trusted_stakers(node: &mut Node) -> usize {
    node.blockchain.states.trusted.stakers.len()
}
