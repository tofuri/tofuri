use crate::Node;
use crate::CARGO_PKG_NAME;
use crate::CARGO_PKG_REPOSITORY;
use crate::CARGO_PKG_VERSION;
use colored::*;
use libp2p::Multiaddr;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tofuri_api_internal_core::Data;
use tofuri_api_internal_core::Data::Address;
use tofuri_api_internal_core::Data::Balance;
use tofuri_api_internal_core::Data::BalancePendingMax;
use tofuri_api_internal_core::Data::BalancePendingMin;
use tofuri_api_internal_core::Data::BlockByHash;
use tofuri_api_internal_core::Data::BlockLatest;
use tofuri_api_internal_core::Data::CargoPkgName;
use tofuri_api_internal_core::Data::CargoPkgRepository;
use tofuri_api_internal_core::Data::CargoPkgVersion;
use tofuri_api_internal_core::Data::DynamicHashes;
use tofuri_api_internal_core::Data::DynamicLatestHashes;
use tofuri_api_internal_core::Data::DynamicStakers;
use tofuri_api_internal_core::Data::GitHash;
use tofuri_api_internal_core::Data::HashByHeight;
use tofuri_api_internal_core::Data::Height;
use tofuri_api_internal_core::Data::HeightByHash;
use tofuri_api_internal_core::Data::Lag;
use tofuri_api_internal_core::Data::Peer;
use tofuri_api_internal_core::Data::Peers;
use tofuri_api_internal_core::Data::RandomQueue;
use tofuri_api_internal_core::Data::Stake;
use tofuri_api_internal_core::Data::StakeByHash;
use tofuri_api_internal_core::Data::Staked;
use tofuri_api_internal_core::Data::StakedPendingMax;
use tofuri_api_internal_core::Data::StakedPendingMin;
use tofuri_api_internal_core::Data::Sync;
use tofuri_api_internal_core::Data::Ticks;
use tofuri_api_internal_core::Data::Time;
use tofuri_api_internal_core::Data::Transaction;
use tofuri_api_internal_core::Data::TransactionByHash;
use tofuri_api_internal_core::Data::TreeSize;
use tofuri_api_internal_core::Data::TrustedHashes;
use tofuri_api_internal_core::Data::TrustedLatestHashes;
use tofuri_api_internal_core::Data::TrustedStakers;
use tofuri_api_internal_core::Request;
use tofuri_block::BlockA;
use tofuri_core::*;
use tofuri_db as db;
use tofuri_p2p::multiaddr;
use tofuri_stake::StakeA;
use tofuri_stake::StakeB;
use tofuri_transaction::TransactionA;
use tofuri_transaction::TransactionB;
use tofuri_util::GIT_HASH;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio::time::Instant;
use tracing::error;
use tracing::info;
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
            BalancePendingMin => bincode::serialize(&balance_pending_min(node, &request.vec)?),
            BalancePendingMax => bincode::serialize(&balance_pending_max(node, &request.vec)?),
            Staked => bincode::serialize(&staked(node, &request.vec)?),
            StakedPendingMin => bincode::serialize(&staked_pending_min(node, &request.vec)?),
            StakedPendingMax => bincode::serialize(&staked_pending_max(node, &request.vec)?),
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
    Ok(node.blockchain.balance(&address_bytes))
}
fn balance_pending_min(node: &mut Node, bytes: &[u8]) -> Result<u128, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    Ok(node.blockchain.balance_pending_min(&address_bytes))
}
fn balance_pending_max(node: &mut Node, bytes: &[u8]) -> Result<u128, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    Ok(node.blockchain.balance_pending_max(&address_bytes))
}
fn staked(node: &mut Node, bytes: &[u8]) -> Result<u128, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    Ok(node.blockchain.staked(&address_bytes))
}
fn staked_pending_min(node: &mut Node, bytes: &[u8]) -> Result<u128, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    Ok(node.blockchain.staked_pending_min(&address_bytes))
}
fn staked_pending_max(node: &mut Node, bytes: &[u8]) -> Result<u128, Box<dyn Error>> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes)?;
    Ok(node.blockchain.staked_pending_max(&address_bytes))
}
fn height(node: &mut Node) -> Result<usize, Box<dyn Error>> {
    Ok(node.blockchain.height())
}
fn height_by_hash(node: &mut Node, bytes: &[u8]) -> Result<usize, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(bytes)?;
    Ok(node.blockchain.height_by_hash(&hash).ok_or("GET HEIGHT_BY_HASH")?)
}
fn block_latest(node: &mut Node) -> Result<&BlockA, Box<dyn Error>> {
    Ok(&node.blockchain.forks.dynamic.latest_block)
}
fn hash_by_height(node: &mut Node, bytes: &[u8]) -> Result<Hash, Box<dyn Error>> {
    let height: usize = bincode::deserialize(bytes)?;
    Ok(node.blockchain.hash_by_height(height).ok_or("GET HASH_BY_HEIGHT")?)
}
fn block_by_hash(node: &mut Node, bytes: &[u8]) -> Result<BlockA, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(bytes)?;
    db::block::get_a(&node.db, &hash)
}
fn transaction_by_hash(node: &mut Node, bytes: &[u8]) -> Result<TransactionA, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(bytes)?;
    db::transaction::get_a(&node.db, &hash)
}
fn stake_by_hash(node: &mut Node, bytes: &[u8]) -> Result<StakeA, Box<dyn Error>> {
    let hash: Hash = bincode::deserialize(bytes)?;
    db::stake::get_a(&node.db, &hash)
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
    CARGO_PKG_NAME
}
fn cargo_pkg_version() -> &'static str {
    CARGO_PKG_VERSION
}
fn cargo_pkg_repository() -> &'static str {
    CARGO_PKG_REPOSITORY
}
fn git_hash() -> &'static str {
    GIT_HASH
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
fn sync(node: &mut Node) -> &tofuri_sync::Sync {
    &node.blockchain.sync
}
fn random_queue(node: &mut Node) -> Vec<AddressBytes> {
    node.blockchain.forks.dynamic.stakers_n(8)
}
fn dynamic_hashes(node: &mut Node) -> usize {
    node.blockchain.forks.dynamic.hashes.len()
}
fn dynamic_latest_hashes(node: &mut Node) -> Vec<&Hash> {
    node.blockchain.forks.dynamic.hashes.iter().rev().take(16).collect()
}
fn dynamic_stakers(node: &mut Node) -> usize {
    node.blockchain.forks.dynamic.stakers.len()
}
fn trusted_hashes(node: &mut Node) -> usize {
    node.blockchain.forks.trusted.hashes.len()
}
fn trusted_latest_hashes(node: &mut Node) -> Vec<&Hash> {
    node.blockchain.forks.trusted.hashes.iter().rev().take(16).collect()
}
fn trusted_stakers(node: &mut Node) -> usize {
    node.blockchain.forks.trusted.stakers.len()
}
