use multiaddr::Multiaddr;
use pea_api_internal_core::Data;
use pea_api_internal_core::Request;
use pea_block::BlockA;
use pea_core::*;
use pea_stake::StakeA;
use pea_stake::StakeB;
use pea_transaction::TransactionA;
use pea_transaction::TransactionB;
use std::error::Error;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
async fn r(data: Data, addr: &str, vec: Option<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    let request = Request {
        data,
        vec: vec.unwrap_or(vec![]),
    };
    stream.write_all(&bincode::serialize(&request)?).await?;
    let mut buf = [0; 1024];
    let bytes = stream.read(&mut buf).await?;
    let vec = buf[..bytes].to_vec();
    Ok(vec)
}
pub async fn balance(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Balance, addr, Some(bincode::serialize(address_bytes)?)).await?)?)
}
pub async fn staked(addr: &str, address_bytes: &AddressBytes) -> Result<u128, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Staked, addr, Some(bincode::serialize(address_bytes)?)).await?)?)
}
pub async fn height(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Height, addr, None).await?)?)
}
pub async fn height_by_hash(addr: &str, hash: &Hash) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::HeightByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn block_latest(addr: &str) -> Result<BlockA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::BlockLatest, addr, None).await?)?)
}
pub async fn hash_by_height(addr: &str, height: &usize) -> Result<Hash, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::HashByHeight, addr, Some(bincode::serialize(height)?)).await?)?)
}
pub async fn block_by_hash(addr: &str, hash: &Hash) -> Result<BlockA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::BlockByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn transaction_by_hash(addr: &str, hash: &Hash) -> Result<TransactionA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::TransactionByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn stake_by_hash(addr: &str, hash: &Hash) -> Result<StakeA, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::StakeByHash, addr, Some(bincode::serialize(hash)?)).await?)?)
}
pub async fn peers(addr: &str) -> Result<Vec<Multiaddr>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Peers, addr, None).await?)?)
}
pub async fn peer(addr: &str, multiaddr: &Multiaddr) -> Result<(), Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Peer, addr, Some(bincode::serialize(multiaddr)?)).await?)?)
}
pub async fn transaction(addr: &str, transaction_b: &TransactionB) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(
        &r(Data::Transaction, addr, Some(bincode::serialize(transaction_b)?)).await?,
    )?)
}
pub async fn stake(addr: &str, stake_b: &StakeB) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Stake, addr, Some(bincode::serialize(stake_b)?)).await?)?)
}
pub async fn cargo_pkg_name(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::CargoPkgName, addr, None).await?)?)
}
pub async fn cargo_pkg_version(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::CargoPkgVersion, addr, None).await?)?)
}
pub async fn cargo_pkg_repository(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::CargoPkgRepository, addr, None).await?)?)
}
pub async fn git_hash(addr: &str) -> Result<String, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::GitHash, addr, None).await?)?)
}
pub async fn address(addr: &str) -> Result<AddressBytes, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Address, addr, None).await?)?)
}
pub async fn ticks(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Ticks, addr, None).await?)?)
}
pub async fn lag(addr: &str) -> Result<f64, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Lag, addr, None).await?)?)
}
pub async fn time(addr: &str) -> Result<i64, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Time, addr, None).await?)?)
}
pub async fn tree_size(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::TreeSize, addr, None).await?)?)
}
pub async fn sync(addr: &str) -> Result<pea_blockchain::sync::Sync, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::Sync, addr, None).await?)?)
}
pub async fn random_queue(addr: &str) -> Result<Vec<AddressBytes>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::RandomQueue, addr, None).await?)?)
}
pub async fn dynamic_hashes(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::DynamicHashes, addr, None).await?)?)
}
pub async fn dynamic_latest_hashes(addr: &str) -> Result<Vec<Hash>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::DynamicLatestHashes, addr, None).await?)?)
}
pub async fn dynamic_stakers(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::DynamicStakers, addr, None).await?)?)
}
pub async fn trusted_hashes(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::TrustedHashes, addr, None).await?)?)
}
pub async fn trusted_latest_hashes(addr: &str) -> Result<Vec<Hash>, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::TrustedLatestHashes, addr, None).await?)?)
}
pub async fn trusted_stakers(addr: &str) -> Result<usize, Box<dyn Error>> {
    Ok(bincode::deserialize(&r(Data::TrustedStakers, addr, None).await?)?)
}
