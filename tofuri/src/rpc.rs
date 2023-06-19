use crate::Node;
use crate::CARGO_PKG_NAME;
use crate::CARGO_PKG_REPOSITORY;
use crate::CARGO_PKG_VERSION;
use std::io;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::time::Duration;
use tofuri_block::Block;
use tofuri_db as db;
use tofuri_rpc_core::Request;
use tofuri_rpc_core::Type;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use tofuri_util::GIT_HASH;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::debug;
use tracing::error;
use tracing::instrument;
#[derive(Debug)]
pub enum Error {
    Blockchain(tofuri_blockchain::Error),
    DBBlock(tofuri_db::block::Error),
    DBTransaction(tofuri_db::transaction::Error),
    DBStake(tofuri_db::stake::Error),
    Bincode(bincode::Error),
    Io(std::io::Error),
    Elapsed(tokio::time::error::Elapsed),
}
#[instrument(skip_all, level = "debug")]
pub async fn accept(node: &mut Node, res: Result<(TcpStream, SocketAddr), io::Error>) {
    match res {
        Ok((stream, socket_addr)) => match request(node, stream).await {
            Ok((bytes, t)) => debug!(?socket_addr, bytes, ?t),
            Err(e) => error!(?e, ?socket_addr),
        },
        Err(e) => error!(?e),
    }
}
#[instrument(skip_all, level = "trace")]
async fn request(node: &mut Node, mut stream: TcpStream) -> Result<(usize, Type), Error> {
    let mut buf = [0; 1024];
    let bytes = timeout(Duration::from_millis(1), stream.read(&mut buf))
        .await
        .map_err(Error::Elapsed)?
        .map_err(Error::Io)?;
    let slice = &buf[..bytes];
    let request: Request = bincode::deserialize(slice).map_err(Error::Bincode)?;
    stream
        .write_all(&match request.t {
            Type::Balance => {
                bincode::serialize(&balance(node, &request.v)?).map_err(Error::Bincode)?
            }
            Type::BalancePendingMin => bincode::serialize(&balance_pending_min(node, &request.v)?)
                .map_err(Error::Bincode)?,
            Type::BalancePendingMax => bincode::serialize(&balance_pending_max(node, &request.v)?)
                .map_err(Error::Bincode)?,
            Type::Staked => {
                bincode::serialize(&staked(node, &request.v)?).map_err(Error::Bincode)?
            }
            Type::StakedPendingMin => bincode::serialize(&staked_pending_min(node, &request.v)?)
                .map_err(Error::Bincode)?,
            Type::StakedPendingMax => bincode::serialize(&staked_pending_max(node, &request.v)?)
                .map_err(Error::Bincode)?,
            Type::Height => bincode::serialize(&height(node)?).map_err(Error::Bincode)?,
            Type::HeightByHash => {
                bincode::serialize(&height_by_hash(node, &request.v)?).map_err(Error::Bincode)?
            }
            Type::BlockLatest => bincode::serialize(block_latest(node)?).map_err(Error::Bincode)?,
            Type::HashByHeight => {
                bincode::serialize(&hash_by_height(node, &request.v)?).map_err(Error::Bincode)?
            }
            Type::BlockByHash => {
                bincode::serialize(&block_by_hash(node, &request.v)?).map_err(Error::Bincode)?
            }
            Type::TransactionByHash => bincode::serialize(&transaction_by_hash(node, &request.v)?)
                .map_err(Error::Bincode)?,
            Type::StakeByHash => {
                bincode::serialize(&stake_by_hash(node, &request.v)?).map_err(Error::Bincode)?
            }
            Type::Peers => bincode::serialize(&peers(node)?).map_err(Error::Bincode)?,
            Type::Peer => bincode::serialize(&peer(node, &request.v)?).map_err(Error::Bincode)?,
            Type::Transaction => {
                bincode::serialize(&transaction(node, &request.v)?).map_err(Error::Bincode)?
            }
            Type::Stake => bincode::serialize(&stake(node, &request.v)?).map_err(Error::Bincode)?,
            Type::CargoPkgName => bincode::serialize(cargo_pkg_name()).map_err(Error::Bincode)?,
            Type::CargoPkgVersion => {
                bincode::serialize(cargo_pkg_version()).map_err(Error::Bincode)?
            }
            Type::CargoPkgRepository => {
                bincode::serialize(cargo_pkg_repository()).map_err(Error::Bincode)?
            }
            Type::GitHash => bincode::serialize(git_hash()).map_err(Error::Bincode)?,
            Type::Address => bincode::serialize(&address(node)).map_err(Error::Bincode)?,
            Type::Ticks => bincode::serialize(ticks(node)).map_err(Error::Bincode)?,
            Type::Time => bincode::serialize(&time()).map_err(Error::Bincode)?,
            Type::TreeSize => bincode::serialize(&tree_size(node)).map_err(Error::Bincode)?,
            Type::Sync => bincode::serialize(sync(node)).map_err(Error::Bincode)?,
            Type::RandomQueue => bincode::serialize(&random_queue(node)).map_err(Error::Bincode)?,
            Type::UnstableHashes => {
                bincode::serialize(&unstable_hashes(node)).map_err(Error::Bincode)?
            }
            Type::UnstableLatestHashes => {
                bincode::serialize(&unstable_latest_hashes(node)).map_err(Error::Bincode)?
            }
            Type::UnstableStakers => {
                bincode::serialize(&unstable_stakers(node)).map_err(Error::Bincode)?
            }
            Type::StableHashes => {
                bincode::serialize(&stable_hashes(node)).map_err(Error::Bincode)?
            }
            Type::StableLatestHashes => {
                bincode::serialize(&stable_latest_hashes(node)).map_err(Error::Bincode)?
            }
            Type::StableStakers => {
                bincode::serialize(&stable_stakers(node)).map_err(Error::Bincode)?
            }
        })
        .await
        .map_err(Error::Io)?;
    stream.flush().await.map_err(Error::Io)?;
    Ok((bytes, request.t))
}
#[instrument(skip_all, level = "trace")]
fn balance(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: [u8; 20] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let balance = node.blockchain.balance(&address_bytes);
    Ok(balance)
}
#[instrument(skip_all, level = "trace")]
fn balance_pending_min(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: [u8; 20] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let balance_pending_min = node.blockchain.balance_pending_min(&address_bytes);
    Ok(balance_pending_min)
}
#[instrument(skip_all, level = "trace")]
fn balance_pending_max(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: [u8; 20] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let balance_pending_max = node.blockchain.balance_pending_max(&address_bytes);
    Ok(balance_pending_max)
}
#[instrument(skip_all, level = "trace")]
fn staked(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: [u8; 20] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let staked = node.blockchain.staked(&address_bytes);
    Ok(staked)
}
#[instrument(skip_all, level = "trace")]
fn staked_pending_min(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: [u8; 20] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let staked_pending_min = node.blockchain.staked_pending_min(&address_bytes);
    Ok(staked_pending_min)
}
#[instrument(skip_all, level = "trace")]
fn staked_pending_max(node: &mut Node, bytes: &[u8]) -> Result<u128, Error> {
    let address_bytes: [u8; 20] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let staked_pending_max = node.blockchain.staked_pending_max(&address_bytes);
    Ok(staked_pending_max)
}
#[instrument(skip_all, level = "trace")]
fn height(node: &mut Node) -> Result<usize, Error> {
    let height = node.blockchain.height();
    Ok(height)
}
#[instrument(skip_all, level = "trace")]
fn height_by_hash(node: &mut Node, bytes: &[u8]) -> Result<usize, Error> {
    let hash: [u8; 32] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    node.blockchain
        .height_by_hash(&hash)
        .map_err(Error::Blockchain)
}
#[instrument(skip_all, level = "trace")]
fn block_latest(node: &mut Node) -> Result<&Block, Error> {
    Ok(&node.blockchain.forks.unstable.latest_block)
}
#[instrument(skip_all, level = "trace")]
fn hash_by_height(node: &mut Node, bytes: &[u8]) -> Result<[u8; 32], Error> {
    let height: usize = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    node.blockchain
        .hash_by_height(height)
        .map_err(Error::Blockchain)
}
#[instrument(skip_all, level = "trace")]
fn block_by_hash(node: &mut Node, bytes: &[u8]) -> Result<Block, Error> {
    let hash: [u8; 32] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    db::block::get(&node.db, &hash).map_err(Error::DBBlock)
}
#[instrument(skip_all, level = "trace")]
fn transaction_by_hash(node: &mut Node, bytes: &[u8]) -> Result<Transaction, Error> {
    let hash: [u8; 32] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    db::transaction::get(&node.db, &hash).map_err(Error::DBTransaction)
}
#[instrument(skip_all, level = "trace")]
fn stake_by_hash(node: &mut Node, bytes: &[u8]) -> Result<Stake, Error> {
    let hash: [u8; 32] = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    db::stake::get(&node.db, &hash).map_err(Error::DBStake)
}
#[instrument(skip_all, level = "trace")]
fn peers(node: &mut Node) -> Result<Vec<&IpAddr>, Error> {
    let vec = node.p2p.connections.values().collect();
    Ok(vec)
}
#[instrument(skip_all, level = "trace")]
fn peer(node: &mut Node, bytes: &[u8]) -> Result<(), Error> {
    let ip_addr = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    node.p2p.connections_unknown.insert(ip_addr);
    Ok(())
}
#[instrument(skip_all, level = "trace")]
fn transaction(node: &mut Node, bytes: &[u8]) -> Result<String, Error> {
    let transaction_b: Transaction = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let vec = bincode::serialize(&transaction_b).unwrap();
    let status = match node
        .blockchain
        .pending_transactions_push(transaction_b, node.args.time_delta)
    {
        Ok(()) => {
            if let Err(e) = node.p2p.gossipsub_publish("transaction", vec) {
                error!(?e);
            }
            "success".to_string()
        }
        Err(e) => {
            error!(?e);
            format!("{:?}", e)
        }
    };
    Ok(status)
}
#[instrument(skip_all, level = "trace")]
fn stake(node: &mut Node, bytes: &[u8]) -> Result<String, Error> {
    let stake_b: Stake = bincode::deserialize(bytes).map_err(Error::Bincode)?;
    let vec = bincode::serialize(&stake_b).unwrap();
    let status = match node
        .blockchain
        .pending_stakes_push(stake_b, node.args.time_delta)
    {
        Ok(()) => {
            if let Err(e) = node.p2p.gossipsub_publish("stake", vec) {
                error!(?e);
            }
            "success".to_string()
        }
        Err(e) => {
            error!(?e);
            format!("{:?}", e)
        }
    };
    Ok(status)
}
#[instrument(skip_all, level = "trace")]
fn cargo_pkg_name() -> &'static str {
    CARGO_PKG_NAME
}
#[instrument(skip_all, level = "trace")]
fn cargo_pkg_version() -> &'static str {
    CARGO_PKG_VERSION
}
#[instrument(skip_all, level = "trace")]
fn cargo_pkg_repository() -> &'static str {
    CARGO_PKG_REPOSITORY
}
#[instrument(skip_all, level = "trace")]
fn git_hash() -> &'static str {
    GIT_HASH
}
#[instrument(skip_all, level = "trace")]
fn address(node: &mut Node) -> Option<[u8; 20]> {
    node.key.as_ref().map(|x| x.address_bytes())
}
#[instrument(skip_all, level = "trace")]
fn ticks(node: &mut Node) -> &usize {
    &node.ticks
}
#[instrument(skip_all, level = "trace")]
fn time() -> i64 {
    chrono::offset::Utc::now().timestamp_millis()
}
#[instrument(skip_all, level = "trace")]
fn tree_size(node: &mut Node) -> usize {
    node.blockchain.tree.size()
}
#[instrument(skip_all, level = "trace")]
fn sync(node: &mut Node) -> &tofuri_sync::Sync {
    &node.blockchain.sync
}
#[instrument(skip_all, level = "trace")]
fn random_queue(node: &mut Node) -> Vec<[u8; 20]> {
    node.blockchain.forks.unstable.stakers_n(8)
}
#[instrument(skip_all, level = "trace")]
fn unstable_hashes(node: &mut Node) -> usize {
    node.blockchain.forks.unstable.hashes.len()
}
#[instrument(skip_all, level = "trace")]
fn unstable_latest_hashes(node: &mut Node) -> Vec<&[u8; 32]> {
    node.blockchain
        .forks
        .unstable
        .hashes
        .iter()
        .rev()
        .take(16)
        .collect()
}
#[instrument(skip_all, level = "trace")]
fn unstable_stakers(node: &mut Node) -> usize {
    node.blockchain.forks.unstable.stakers.len()
}
#[instrument(skip_all, level = "trace")]
fn stable_hashes(node: &mut Node) -> usize {
    node.blockchain.forks.stable.hashes.len()
}
#[instrument(skip_all, level = "trace")]
fn stable_latest_hashes(node: &mut Node) -> Vec<&[u8; 32]> {
    node.blockchain
        .forks
        .stable
        .hashes
        .iter()
        .rev()
        .take(16)
        .collect()
}
#[instrument(skip_all, level = "trace")]
fn stable_stakers(node: &mut Node) -> usize {
    node.blockchain.forks.stable.stakers.len()
}
