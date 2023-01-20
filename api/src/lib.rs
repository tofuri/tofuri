use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
pub type Index = String;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Info {
    pub time: String,
    pub address: String,
    pub uptime: String,
    pub heartbeats: usize,
    pub tree_size: usize,
    pub lag: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sync {
    pub status: String,
    pub height: usize,
    pub last_seen: String,
}
pub type Height = usize;
pub type Amount = String;
pub type Hash = String;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Dynamic {
    pub random_queue: Vec<String>,
    pub hashes: usize,
    pub latest_hashes: Vec<String>,
    pub stakers: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trusted {
    pub hashes: usize,
    pub latest_hashes: Vec<String>,
    pub stakers: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Options {
    pub mint: bool,
    pub tempdb: bool,
    pub tempkey: bool,
    pub trust: usize,
    pub ban_offline: usize,
    pub time_delta: u32,
    pub max_established: Option<u32>,
    pub tps: f64,
    pub bind_api: String,
    pub host: String,
    pub dev: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: u32,
    pub address: String,
    pub signature: String,
    pub pi: String,
    pub beta: String,
    pub transactions: Vec<String>,
    pub stakes: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub hash: String,
    pub input_address: String,
    pub output_address: String,
    pub amount: Amount,
    pub fee: Amount,
    pub timestamp: u32,
    pub signature: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stake {
    pub hash: String,
    pub address: String,
    pub fee: Amount,
    pub deposit: bool,
    pub timestamp: u32,
    pub signature: String,
}
enum Method {
    GET,
    POST,
}
fn request_line(method: Method, path: &str) -> String {
    format!(
        "{} {} HTTP/1.1",
        match method {
            Method::GET => "GET",
            Method::POST => "POST",
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
    pub async fn index(api: &str) -> Result<Index, Box<dyn Error>> {
        request(api, Method::GET, "/", None).await
    }
    pub async fn info(api: &str) -> Result<Info, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/info", None).await?)?)
    }
    pub async fn sync(api: &str) -> Result<Sync, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/sync", None).await?)?)
    }
    pub async fn height(api: &str) -> Result<Height, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/height", None).await?)?)
    }
    pub async fn balance(api: &str, address: &str) -> Result<Amount, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/balance/{address}"), None).await?)?)
    }
    pub async fn staked(api: &str, address: &str) -> Result<Amount, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/staked/{address}"), None).await?)?)
    }
    pub async fn hash(api: &str, height: &usize) -> Result<Hash, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/hash/{height}"), None).await?)?)
    }
    pub async fn dynamic(api: &str) -> Result<Dynamic, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/dynamic", None).await?)?)
    }
    pub async fn trusted(api: &str) -> Result<Trusted, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/trusted", None).await?)?)
    }
    pub async fn options(api: &str) -> Result<Options, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/options", None).await?)?)
    }
    pub async fn block(api: &str, hash: &str) -> Result<Block, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/block/{hash}"), None).await?)?)
    }
    pub async fn latest_block(api: &str) -> Result<Block, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/block/latest", None).await?)?)
    }
    pub async fn transaction(api: &str, hash: &str) -> Result<Transaction, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/transaction/{hash}"), None).await?)?)
    }
    pub async fn stake(api: &str, hash: &str) -> Result<Stake, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/stake/{hash}"), None).await?)?)
    }
}
pub mod post {
    use super::*;
    use pea_stake::StakeB;
    use pea_transaction::TransactionB;
    use std::error::Error;
    pub async fn transaction(api: &str, transaction_b: &TransactionB) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::POST, "/transaction", Some(&serde_json::to_string(transaction_b)?)).await?,
        )?)
    }
    pub async fn stake(api: &str, stake_b: &StakeB) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::POST, "/stake", Some(&serde_json::to_string(stake_b)?)).await?,
        )?)
    }
}
