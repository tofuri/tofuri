use crate::Error;
use crate::Node;
use crate::CARGO_PKG_NAME;
use crate::CARGO_PKG_REPOSITORY;
use crate::CARGO_PKG_VERSION;
use colored::*;
use std::io;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::time::Duration;
use tofuri_block::BlockA;
use tofuri_core::*;
use tofuri_db as db;
use tofuri_rpc_core::Request;
use tofuri_rpc_core::Type;
use tofuri_stake::StakeA;
use tofuri_stake::StakeB;
use tofuri_transaction::TransactionA;
use tofuri_transaction::TransactionB;
use tofuri_util::GIT_HASH;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::error;
use tracing::info;
#[tracing::instrument(skip_all, level = "debug")]
pub async fn accept(node: &mut Node, res: Result<(TcpStream, SocketAddr), io::Error>) {
    if let Err(err) = &res {
        error!("{}", err.to_string().red());
    }
    let (stream, socket_addr) = res.unwrap();
    match request(node, stream).await {
        Ok((bytes, t)) => info!(socket_addr = socket_addr.to_string(), bytes, "{}", format!("{:?}", t).magenta()),
        Err(err) => error!(socket_addr = socket_addr.to_string(), "{:?}", err),
    };
}
#[tracing::instrument(skip_all, level = "trace")]
async fn request(node: &mut Node, mut stream: TcpStream) -> Result<(usize, Type), Error> {
    let mut buf = [0; 1024];
    let bytes = timeout(Duration::from_millis(1), stream.read(&mut buf))
        .await
        .map_err(Error::Elapsed)?
        .map_err(Error::Io)?;
    let slice = &buf[..bytes];
    let request: Request = bincode::deserialize(slice).map_err(Error::Bincode)?;
    stream
        .write_all(
            &match request.t {
                Type::Balance => bincode::serialize(&balance(node, &request.v)?),
                Type::BalancePendingMin => bincode::serialize(&balance_pending_min(node, &request.v)?),
                Type::BalancePendingMax => bincode::serialize(&balance_pending_max(node, &request.v)?),
                Type::Staked => bincode::serialize(&staked(node, &request.v)?),
                Type::StakedPendingMin => bincode::serialize(&staked_pending_min(node, &request.v)?),
                Type::StakedPendingMax => bincode::serialize(&staked_pending_max(node, &request.v)?),
                Type::Height => bincode::serialize(&height(node)?),
                Type::HeightByHash => bincode::serialize(&height_by_hash(node, &request.v)?),
                Type::BlockLatest => bincode::serialize(block_latest(node)?),
                Type::HashByHeight => bincode::serialize(&hash_by_height(node, &request.v)?),
                Type::BlockByHash => bincode::serialize(&block_by_hash(node, &request.v)?),
                Type::TransactionByHash => bincode::serialize(&transaction_by_hash(node, &request.v)?),
                Type::StakeByHash => bincode::serialize(&stake_by_hash(node, &request.v)?),
                Type::Peers => bincode::serialize(&peers(node)?),
                Type::Peer => bincode::serialize(&peer(node, &request.v)?),
                Type::Transaction => bincode::serialize(&transaction(node, &request.v)?),
                Type::Stake => bincode::serialize(&stake(node, &request.v)?),
                Type::CargoPkgName => bincode::serialize(cargo_pkg_name()),
                Type::CargoPkgVersion => bincode::serialize(cargo_pkg_version()),
                Type::CargoPkgRepository => bincode::serialize(cargo_pkg_repository()),
                Type::GitHash => bincode::serialize(git_hash()),
                Type::Address => bincode::serialize(&address(node)),
                Type::Ticks => bincode::serialize(ticks(node)),
                Type::Time => bincode::serialize(&time()),
                Type::TreeSize => bincode::serialize(&tree_size(node)),
                Type::Sync => bincode::serialize(sync(node)),
                Type::RandomQueue => bincode::serialize(&random_queue(node)),
                Type::UnstableHashes => bincode::serialize(&unstable_hashes(node)),
                Type::UnstableLatestHashes => bincode::serialize(&unstable_latest_hashes(node)),
                Type::UnstableStakers => bincode::serialize(&unstable_stakers(node)),
                Type::StableHashes => bincode::serialize(&stable_hashes(node)),
                Type::StableLatestHashes => bincode::serialize(&stable_latest_hashes(node)),
                Type::StableStakers => bincode::serialize(&stable_stakers(node)),
            }
            .map_err(Error::Bincode)?,
        )
        .await
        .map_err(Error::Io)?;
    stream.flush().await.map_err(Error::Io)?;
    Ok((bytes, request.t))
}
#[tracing::instrument(skip_all, level = "trace")]
fn balance(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.balance(&address_bytes))
}
#[tracing::instrument(skip_all, level = "trace")]
fn balance_pending_min(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.balance_pending_min(&address_bytes))
}
#[tracing::instrument(skip_all, level = "trace")]
fn balance_pending_max(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.balance_pending_max(&address_bytes))
}
#[tracing::instrument(skip_all, level = "trace")]
fn staked(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.staked(&address_bytes))
}
#[tracing::instrument(skip_all, level = "trace")]
fn staked_pending_min(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.staked_pending_min(&address_bytes))
}
#[tracing::instrument(skip_all, level = "trace")]
fn staked_pending_max(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: AddressBytes = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.staked_pending_max(&address_bytes))
}
#[tracing::instrument(skip_all, level = "trace")]
fn height(node: &mut Node) -> Result<usize, Error> {
    Ok(node.blockchain.height())
}
#[tracing::instrument(skip_all, level = "trace")]
fn height_by_hash(node: &mut Node, bytes: &[u8]) -> Result<usize, Error> {
    let hash: Hash = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.height_by_hash(&hash).map_err(Error::Blockchain)?)
}
#[tracing::instrument(skip_all, level = "trace")]
fn block_latest(node: &mut Node) -> Result<&BlockA, Error> {
    Ok(&node.blockchain.forks.unstable.latest_block)
}
#[tracing::instrument(skip_all, level = "trace")]
fn hash_by_height(node: &mut Node, bytes: &[u8]) -> Result<Hash, Error> {
    let height: usize = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    Ok(node.blockchain.hash_by_height(height).map_err(Error::Blockchain)?)
}
#[tracing::instrument(skip_all, level = "trace")]
fn block_by_hash(node: &mut Node, bytes: &[u8]) -> Result<BlockA, Error> {
    let hash: Hash = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    db::block::get_a(&node.db, &hash).map_err(Error::DB)
}
#[tracing::instrument(skip_all, level = "trace")]
fn transaction_by_hash(node: &mut Node, bytes: &[u8]) -> Result<TransactionA, Error> {
    let hash: Hash = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    db::transaction::get_a(&node.db, &hash).map_err(Error::DB)
}
#[tracing::instrument(skip_all, level = "trace")]
fn stake_by_hash(node: &mut Node, bytes: &[u8]) -> Result<StakeA, Error> {
    let hash: Hash = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    db::stake::get_a(&node.db, &hash).map_err(Error::DB)
}
#[tracing::instrument(skip_all, level = "trace")]
fn peers(node: &mut Node) -> Result<Vec<&IpAddr>, Error> {
    Ok(node.p2p.connections.values().collect())
}
#[tracing::instrument(skip_all, level = "trace")]
fn peer(node: &mut Node, bytes: &[u8]) -> Result<(), Error> {
    let ip_addr = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    node.p2p.unknown.insert(ip_addr);
    Ok(())
}
#[tracing::instrument(skip_all, level = "trace")]
fn transaction(node: &mut Node, bytes: &[u8]) -> Result<String, Error> {
    let transaction_b: TransactionB = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let vec = bincode::serialize(&transaction_b).unwrap();
    let status = match node.blockchain.pending_transactions_push(transaction_b, node.args.time_delta) {
        Ok(()) => {
            if let Err(err) = node.p2p.gossipsub_publish("transaction", vec) {
                error!("{:?}", err);
            }
            "success".to_string()
        }
        Err(err) => {
            error!("{:?}", err);
            format!("{:?}", err)
        }
    };
    Ok(status)
}
#[tracing::instrument(skip_all, level = "trace")]
fn stake(node: &mut Node, bytes: &[u8]) -> Result<String, Error> {
    let stake_b: StakeB = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let vec = bincode::serialize(&stake_b).unwrap();
    let status = match node.blockchain.pending_stakes_push(stake_b, node.args.time_delta) {
        Ok(()) => {
            if let Err(err) = node.p2p.gossipsub_publish("stake", vec) {
                error!("{:?}", err);
            }
            "success".to_string()
        }
        Err(err) => {
            error!("{:?}", err);
            format!("{:?}", err)
        }
    };
    Ok(status)
}
#[tracing::instrument(skip_all, level = "trace")]
fn cargo_pkg_name() -> &'static str {
    CARGO_PKG_NAME
}
#[tracing::instrument(skip_all, level = "trace")]
fn cargo_pkg_version() -> &'static str {
    CARGO_PKG_VERSION
}
#[tracing::instrument(skip_all, level = "trace")]
fn cargo_pkg_repository() -> &'static str {
    CARGO_PKG_REPOSITORY
}
#[tracing::instrument(skip_all, level = "trace")]
fn git_hash() -> &'static str {
    GIT_HASH
}
#[tracing::instrument(skip_all, level = "trace")]
fn address(node: &mut Node) -> AddressBytes {
    node.key.address_bytes()
}
#[tracing::instrument(skip_all, level = "trace")]
fn ticks(node: &mut Node) -> &usize {
    &node.ticks
}
#[tracing::instrument(skip_all, level = "trace")]
fn time() -> i64 {
    chrono::offset::Utc::now().timestamp_millis()
}
#[tracing::instrument(skip_all, level = "trace")]
fn tree_size(node: &mut Node) -> usize {
    node.blockchain.tree.size()
}
#[tracing::instrument(skip_all, level = "trace")]
fn sync(node: &mut Node) -> &tofuri_sync::Sync {
    &node.blockchain.sync
}
#[tracing::instrument(skip_all, level = "trace")]
fn random_queue(node: &mut Node) -> Vec<AddressBytes> {
    node.blockchain.forks.unstable.stakers_n(8)
}
#[tracing::instrument(skip_all, level = "trace")]
fn unstable_hashes(node: &mut Node) -> usize {
    node.blockchain.forks.unstable.hashes.len()
}
#[tracing::instrument(skip_all, level = "trace")]
fn unstable_latest_hashes(node: &mut Node) -> Vec<&Hash> {
    node.blockchain.forks.unstable.hashes.iter().rev().take(16).collect()
}
#[tracing::instrument(skip_all, level = "trace")]
fn unstable_stakers(node: &mut Node) -> usize {
    node.blockchain.forks.unstable.stakers.len()
}
#[tracing::instrument(skip_all, level = "trace")]
fn stable_hashes(node: &mut Node) -> usize {
    node.blockchain.forks.stable.hashes.len()
}
#[tracing::instrument(skip_all, level = "trace")]
fn stable_latest_hashes(node: &mut Node) -> Vec<&Hash> {
    node.blockchain.forks.stable.hashes.iter().rev().take(16).collect()
}
#[tracing::instrument(skip_all, level = "trace")]
fn stable_stakers(node: &mut Node) -> usize {
    node.blockchain.forks.stable.stakers.len()
}
