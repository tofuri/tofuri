use serde::Deserialize;
use serde::Serialize;
use std::net::IpAddr;
use tofuri_block::Block;
use tofuri_stake::Stake;
use tofuri_sync::Sync;
use tofuri_transaction::Transaction;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Type {
    Balance,
    BalancePendingMin,
    BalancePendingMax,
    Staked,
    StakedPendingMin,
    StakedPendingMax,
    Height,
    HeightByHash,
    BlockLatest,
    HashByHeight,
    BlockByHash,
    TransactionByHash,
    StakeByHash,
    Peers,
    Peer,
    Transaction,
    Stake,
    CargoPkgName,
    CargoPkgVersion,
    CargoPkgRepository,
    GitHash,
    Address,
    Ticks,
    Time,
    TreeSize,
    Sync,
    RandomQueue,
    UnstableHashes,
    UnstableLatestHashes,
    UnstableStakers,
    StableHashes,
    StableLatestHashes,
    StableStakers,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub t: Type,
    pub v: Vec<u8>,
}
#[derive(Debug)]
pub enum Error {
    Bincode(bincode::Error),
    Io(std::io::Error),
}
async fn request(t: Type, addr: &str, v: Option<Vec<u8>>) -> Result<Vec<u8>, Error> {
    let mut stream = TcpStream::connect(addr).await.map_err(Error::Io)?;
    let request = Request {
        t,
        v: v.unwrap_or(vec![]),
    };
    stream
        .write_all(&bincode::serialize(&request).map_err(Error::Bincode)?)
        .await
        .map_err(Error::Io)?;
    let mut buf = [0; 1024];
    let bytes = stream.read(&mut buf).await.map_err(Error::Io)?;
    let vec = buf[..bytes].to_vec();
    Ok(vec)
}
pub async fn balance(addr: &str, address_bytes: &[u8; 20]) -> Result<u128, Error> {
    bincode::deserialize(
        &request(
            Type::Balance,
            addr,
            Some(bincode::serialize(address_bytes).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn balance_pending_min(addr: &str, address_bytes: &[u8; 20]) -> Result<u128, Error> {
    bincode::deserialize(
        &request(
            Type::BalancePendingMin,
            addr,
            Some(bincode::serialize(address_bytes).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn balance_pending_max(addr: &str, address_bytes: &[u8; 20]) -> Result<u128, Error> {
    bincode::deserialize(
        &request(
            Type::BalancePendingMax,
            addr,
            Some(bincode::serialize(address_bytes).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn staked(addr: &str, address_bytes: &[u8; 20]) -> Result<u128, Error> {
    bincode::deserialize(
        &request(
            Type::Staked,
            addr,
            Some(bincode::serialize(address_bytes).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn staked_pending_min(addr: &str, address_bytes: &[u8; 20]) -> Result<u128, Error> {
    bincode::deserialize(
        &request(
            Type::StakedPendingMin,
            addr,
            Some(bincode::serialize(address_bytes).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn staked_pending_max(addr: &str, address_bytes: &[u8; 20]) -> Result<u128, Error> {
    bincode::deserialize(
        &request(
            Type::StakedPendingMax,
            addr,
            Some(bincode::serialize(address_bytes).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn height(addr: &str) -> Result<usize, Error> {
    bincode::deserialize(&request(Type::Height, addr, None).await?).map_err(Error::Bincode)
}
pub async fn height_by_hash(addr: &str, hash: &[u8; 32]) -> Result<usize, Error> {
    bincode::deserialize(
        &request(
            Type::HeightByHash,
            addr,
            Some(bincode::serialize(hash).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn block_latest(addr: &str) -> Result<Block, Error> {
    bincode::deserialize(&request(Type::BlockLatest, addr, None).await?).map_err(Error::Bincode)
}
pub async fn hash_by_height(addr: &str, height: &usize) -> Result<[u8; 32], Error> {
    bincode::deserialize(
        &request(
            Type::HashByHeight,
            addr,
            Some(bincode::serialize(height).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn block_by_hash(addr: &str, hash: &[u8; 32]) -> Result<Block, Error> {
    bincode::deserialize(
        &request(
            Type::BlockByHash,
            addr,
            Some(bincode::serialize(hash).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn transaction_by_hash(addr: &str, hash: &[u8; 32]) -> Result<Transaction, Error> {
    bincode::deserialize(
        &request(
            Type::TransactionByHash,
            addr,
            Some(bincode::serialize(hash).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn stake_by_hash(addr: &str, hash: &[u8; 32]) -> Result<Stake, Error> {
    bincode::deserialize(
        &request(
            Type::StakeByHash,
            addr,
            Some(bincode::serialize(hash).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn peers(addr: &str) -> Result<Vec<IpAddr>, Error> {
    bincode::deserialize(&request(Type::Peers, addr, None).await?).map_err(Error::Bincode)
}
pub async fn peer(addr: &str, ip_addr: &IpAddr) -> Result<(), Error> {
    bincode::deserialize(
        &request(
            Type::Peer,
            addr,
            Some(bincode::serialize(ip_addr).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn transaction(addr: &str, transaction_b: &Transaction) -> Result<String, Error> {
    bincode::deserialize(
        &request(
            Type::Transaction,
            addr,
            Some(bincode::serialize(transaction_b).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn stake(addr: &str, stake_b: &Stake) -> Result<String, Error> {
    bincode::deserialize(
        &request(
            Type::Stake,
            addr,
            Some(bincode::serialize(stake_b).map_err(Error::Bincode)?),
        )
        .await?,
    )
    .map_err(Error::Bincode)
}
pub async fn cargo_pkg_name(addr: &str) -> Result<String, Error> {
    bincode::deserialize(&request(Type::CargoPkgName, addr, None).await?).map_err(Error::Bincode)
}
pub async fn cargo_pkg_version(addr: &str) -> Result<String, Error> {
    bincode::deserialize(&request(Type::CargoPkgVersion, addr, None).await?).map_err(Error::Bincode)
}
pub async fn cargo_pkg_repository(addr: &str) -> Result<String, Error> {
    bincode::deserialize(&request(Type::CargoPkgRepository, addr, None).await?)
        .map_err(Error::Bincode)
}
pub async fn git_hash(addr: &str) -> Result<String, Error> {
    bincode::deserialize(&request(Type::GitHash, addr, None).await?).map_err(Error::Bincode)
}
pub async fn address(addr: &str) -> Result<Option<[u8; 20]>, Error> {
    bincode::deserialize(&request(Type::Address, addr, None).await?).map_err(Error::Bincode)
}
pub async fn ticks(addr: &str) -> Result<usize, Error> {
    bincode::deserialize(&request(Type::Ticks, addr, None).await?).map_err(Error::Bincode)
}
pub async fn time(addr: &str) -> Result<i64, Error> {
    bincode::deserialize(&request(Type::Time, addr, None).await?).map_err(Error::Bincode)
}
pub async fn tree_size(addr: &str) -> Result<usize, Error> {
    bincode::deserialize(&request(Type::TreeSize, addr, None).await?).map_err(Error::Bincode)
}
pub async fn sync(addr: &str) -> Result<Sync, Error> {
    bincode::deserialize(&request(Type::Sync, addr, None).await?).map_err(Error::Bincode)
}
pub async fn random_queue(addr: &str) -> Result<Vec<[u8; 20]>, Error> {
    bincode::deserialize(&request(Type::RandomQueue, addr, None).await?).map_err(Error::Bincode)
}
pub async fn unstable_hashes(addr: &str) -> Result<usize, Error> {
    bincode::deserialize(&request(Type::UnstableHashes, addr, None).await?).map_err(Error::Bincode)
}
pub async fn unstable_latest_hashes(addr: &str) -> Result<Vec<[u8; 32]>, Error> {
    bincode::deserialize(&request(Type::UnstableLatestHashes, addr, None).await?)
        .map_err(Error::Bincode)
}
pub async fn unstable_stakers(addr: &str) -> Result<usize, Error> {
    bincode::deserialize(&request(Type::UnstableStakers, addr, None).await?).map_err(Error::Bincode)
}
pub async fn stable_hashes(addr: &str) -> Result<usize, Error> {
    bincode::deserialize(&request(Type::StableHashes, addr, None).await?).map_err(Error::Bincode)
}
pub async fn stable_latest_hashes(addr: &str) -> Result<Vec<[u8; 32]>, Error> {
    bincode::deserialize(&request(Type::StableLatestHashes, addr, None).await?)
        .map_err(Error::Bincode)
}
pub async fn stable_stakers(addr: &str) -> Result<usize, Error> {
    bincode::deserialize(&request(Type::StableStakers, addr, None).await?).map_err(Error::Bincode)
}
