use multiaddr::Multiaddr;
use std::error::Error;
use tofuri_block::BlockA;
use tofuri_core::*;
use tofuri_rpc_core::Request;
use tofuri_rpc_core::Type;
use tofuri_stake::StakeA;
use tofuri_stake::StakeB;
use tofuri_sync::Sync;
use tofuri_transaction::TransactionA;
use tofuri_transaction::TransactionB;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
async fn r(t: Type, addr: &str, v: Option<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    let request = Request { t, v: v.unwrap_or(vec![]) };
    stream.write_all(&bincode::serialize(&request)?).await?;
    let mut buf = [0; 1024];
    let bytes = stream.read(&mut buf).await?;
    let vec = buf[..bytes].to_vec();
    Ok(vec)
}
pub async fn balance(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Balance, addr, Some(bincode::serialize(address_bytes)?)).await?)?)
}
pub async fn balance_pending_min(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(
        &r(Type::BalancePendingMin, addr, Some(bincode::serialize(address_bytes)?)).await?,
    )?)
}
pub async fn balance_pending_max(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(
        &r(Type::BalancePendingMax, addr, Some(bincode::serialize(address_bytes)?)).await?,
    )?)
}
pub async fn staked(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Staked, addr, Some(bincode::serialize(address_bytes)?)).await?)?)
}
pub async fn staked_pending_min(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(
        &r(Type::StakedPendingMin, addr, Some(bincode::serialize(address_bytes)?)).await?,
    )?)
}
pub async fn staked_pending_max(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(
        &r(Type::StakedPendingMax, addr, Some(bincode::serialize(address_bytes)?)).await?,
    )?)
}
pub async fn height(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Height, addr, None).await?)?)
}
pub async fn height_by_hash(addr: &str, hash: &Hash) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::HeightByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn block_latest(addr: &str) -> Result<BlockA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::BlockLatest, addr, None).await?)?)
}
pub async fn hash_by_height(addr: &str, height: &usize) -> Result<Hash, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::HashByHeight, addr, Some(bincode::serialize(height)?)).await?)?)
}
pub async fn block_by_hash(addr: &str, hash: &Hash) -> Result<BlockA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::BlockByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn transaction_by_hash(addr: &str, hash: &Hash) -> Result<TransactionA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::TransactionByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn stake_by_hash(addr: &str, hash: &Hash) -> Result<StakeA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::StakeByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn peers(addr: &str) -> Result<Vec<Multiaddr>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Peers, addr, None).await?)?)
}
pub async fn peer(addr: &str, multiaddr: &Multiaddr) -> Result<(), Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Peer, addr, Some(bincode::serialize(multiaddr)?)).await?)?)
}
pub async fn transaction(addr: &str, transaction_b: &TransactionB) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(
        &r(Type::Transaction, addr, Some(bincode::serialize(transaction_b)?)).await?,
    )?)
}
pub async fn stake(addr: &str, stake_b: &StakeB) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Stake, addr, Some(bincode::serialize(stake_b)?)).await?)?)
}
pub async fn cargo_pkg_name(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::CargoPkgName, addr, None).await?)?)
}
pub async fn cargo_pkg_version(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::CargoPkgVersion, addr, None).await?)?)
}
pub async fn cargo_pkg_repository(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::CargoPkgRepository, addr, None).await?)?)
}
pub async fn git_hash(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::GitHash, addr, None).await?)?)
}
pub async fn address(addr: &str) -> Result<AddressBytes, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Address, addr, None).await?)?)
}
pub async fn ticks(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Ticks, addr, None).await?)?)
}
pub async fn lag(addr: &str) -> Result<f64, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Lag, addr, None).await?)?)
}
pub async fn time(addr: &str) -> Result<i64, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Time, addr, None).await?)?)
}
pub async fn tree_size(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::TreeSize, addr, None).await?)?)
}
pub async fn sync(addr: &str) -> Result<Sync, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::Sync, addr, None).await?)?)
}
pub async fn random_queue(addr: &str) -> Result<Vec<AddressBytes>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::RandomQueue, addr, None).await?)?)
}
pub async fn dynamic_hashes(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::DynamicHashes, addr, None).await?)?)
}
pub async fn dynamic_latest_hashes(addr: &str) -> Result<Vec<Hash>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::DynamicLatestHashes, addr, None).await?)?)
}
pub async fn dynamic_stakers(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::DynamicStakers, addr, None).await?)?)
}
pub async fn trusted_hashes(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::TrustedHashes, addr, None).await?)?)
}
pub async fn trusted_latest_hashes(addr: &str) -> Result<Vec<Hash>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::TrustedLatestHashes, addr, None).await?)?)
}
pub async fn trusted_stakers(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Type::TrustedStakers, addr, None).await?)?)
}
