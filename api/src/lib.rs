use chrono::TimeZone;
use chrono::Utc;
use pea_address::address;
use pea_block::BlockA;
use pea_blockchain::blockchain::Blockchain;
use pea_blockchain::state;
use pea_key::Key;
use pea_stake::StakeA;
use pea_transaction::TransactionA;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Info {
    pub time: String,
    pub address: String,
    pub uptime: String,
    pub ticks: usize,
    pub tree_size: usize,
    pub lag: f64,
}
impl Info {
    pub fn from(key: &Key, ticks: usize, tps: f64, blockchain: &Blockchain, lag: f64) -> Info {
        Info {
            time: Utc.timestamp_nanos(chrono::offset::Utc::now().timestamp_micros() * 1_000).to_rfc2822(),
            address: address::encode(&key.address_bytes()),
            uptime: pea_util::uptime(ticks as f64, tps),
            ticks,
            tree_size: blockchain.tree.size(),
            lag,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Sync {
    pub status: String,
    pub height: usize,
    pub last_seen: String,
}
impl Sync {
    pub fn from(blockchain: &Blockchain) -> Sync {
        let last_seen = blockchain.last_seen();
        let status = blockchain.sync_status();
        let height = blockchain.height();
        Sync { status, last_seen, height }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Dynamic {
    pub random_queue: Vec<String>,
    pub hashes: usize,
    pub latest_hashes: Vec<String>,
    pub stakers: Vec<String>,
}
impl Dynamic {
    pub fn from(dynamic: &state::Dynamic) -> Dynamic {
        Dynamic {
            random_queue: dynamic.stakers_n(8).iter().map(address::encode).collect(),
            hashes: dynamic.hashes.len(),
            latest_hashes: dynamic.hashes.iter().rev().take(16).map(hex::encode).collect(),
            stakers: dynamic.stakers.iter().take(16).map(address::encode).collect(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Trusted {
    pub hashes: usize,
    pub latest_hashes: Vec<String>,
    pub stakers: Vec<String>,
}
impl Trusted {
    pub fn from(trusted: &state::Trusted) -> Trusted {
        Trusted {
            hashes: trusted.hashes.len(),
            latest_hashes: trusted.hashes.iter().rev().take(16).map(hex::encode).collect(),
            stakers: trusted.stakers.iter().take(16).map(address::encode).collect(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Block {
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: u32,
    pub beta: String,
    pub pi: String,
    pub forger_address: String,
    pub signature: String,
    pub transactions: Vec<String>,
    pub stakes: Vec<String>,
}
impl Block {
    pub fn from(block_a: &BlockA) -> Block {
        Block {
            hash: hex::encode(block_a.hash),
            previous_hash: hex::encode(block_a.previous_hash),
            timestamp: block_a.timestamp,
            beta: hex::encode(block_a.beta),
            pi: hex::encode(block_a.pi),
            forger_address: address::encode(&block_a.input_address()),
            signature: hex::encode(block_a.signature),
            transactions: block_a.transactions.iter().map(|x| hex::encode(x.hash)).collect(),
            stakes: block_a.stakes.iter().map(|x| hex::encode(x.hash)).collect(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Transaction {
    pub input_address: String,
    pub output_address: String,
    pub amount: String,
    pub fee: String,
    pub timestamp: u32,
    pub hash: String,
    pub signature: String,
}
impl Transaction {
    pub fn from(transaction_a: &TransactionA) -> Transaction {
        Transaction {
            input_address: address::encode(&transaction_a.input_address),
            output_address: address::encode(&transaction_a.output_address),
            amount: pea_int::to_string(transaction_a.amount),
            fee: pea_int::to_string(transaction_a.fee),
            timestamp: transaction_a.timestamp,
            hash: hex::encode(transaction_a.hash),
            signature: hex::encode(transaction_a.signature),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Stake {
    pub amount: String,
    pub fee: String,
    pub deposit: bool,
    pub timestamp: u32,
    pub signature: String,
    pub input_address: String,
    pub hash: String,
}
impl Stake {
    pub fn from(stake_a: &StakeA) -> Stake {
        Stake {
            amount: pea_int::to_string(stake_a.amount),
            fee: pea_int::to_string(stake_a.fee),
            deposit: stake_a.deposit,
            timestamp: stake_a.timestamp,
            signature: hex::encode(stake_a.signature),
            input_address: address::encode(&stake_a.input_address),
            hash: hex::encode(stake_a.hash),
        }
    }
}
enum Method {
    Get,
    Post,
}
fn request_line(method: Method, path: &str) -> String {
    format!(
        "{} {} HTTP/1.1",
        match method {
            Method::Get => "GET",
            Method::Post => "POST",
        },
        path
    )
}
async fn request(addr: &str, method: Method, path: &str, body: Option<&str>) -> Result<String, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    stream.write_all(request_line(method, path).as_bytes()).await?;
    if let Some(body) = body {
        stream.write_all(b"\n\n").await?;
        stream.write_all(body.as_bytes()).await?;
    }
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).await?;
    parse_body(buffer)
}
fn parse_body(buffer: String) -> Result<String, Box<dyn Error>> {
    let vec = buffer.split("\n\n").collect::<Vec<&str>>();
    let body = vec.get(1).ok_or("empty body")?;
    Ok(body.to_string())
}
pub mod get {
    use super::*;
    use std::error::Error;
    pub async fn index(api: &str) -> Result<String, Box<dyn Error>> {
        request(api, Method::Get, "/", None).await
    }
    pub async fn info(api: &str) -> Result<Info, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, "/info", None).await?)?)
    }
    pub async fn sync(api: &str) -> Result<Sync, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, "/sync", None).await?)?)
    }
    pub async fn height(api: &str) -> Result<usize, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, "/height", None).await?)?)
    }
    pub async fn height_by_hash(api: &str, hash: &str) -> Result<usize, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/height/{hash}"), None).await?)?)
    }
    pub async fn hash_by_height(api: &str, height: usize) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/hash/{height}"), None).await?)?)
    }
    pub async fn balance(api: &str, address: &str) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/balance/{address}"), None).await?)?)
    }
    pub async fn staked(api: &str, address: &str) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/staked/{address}"), None).await?)?)
    }
    pub async fn hash(api: &str, height: &usize) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/hash/{height}"), None).await?)?)
    }
    pub async fn dynamic(api: &str) -> Result<Dynamic, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, "/dynamic", None).await?)?)
    }
    pub async fn trusted(api: &str) -> Result<Trusted, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, "/trusted", None).await?)?)
    }
    pub async fn block(api: &str, hash: &str) -> Result<Block, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/block/{hash}"), None).await?)?)
    }
    pub async fn latest_block(api: &str) -> Result<Block, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, "/block/latest", None).await?)?)
    }
    pub async fn transaction(api: &str, hash: &str) -> Result<Transaction, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/transaction/{hash}"), None).await?)?)
    }
    pub async fn stake(api: &str, hash: &str) -> Result<Stake, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::Get, &format!("/stake/{hash}"), None).await?)?)
    }
}
pub mod post {
    use super::*;
    use pea_stake::StakeB;
    use pea_transaction::TransactionB;
    use std::error::Error;
    pub async fn transaction(api: &str, transaction_b: &TransactionB) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::Post, "/transaction", Some(&serde_json::to_string(transaction_b)?)).await?,
        )?)
    }
    pub async fn stake(api: &str, stake_b: &StakeB) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::Post, "/stake", Some(&serde_json::to_string(stake_b)?)).await?,
        )?)
    }
}
