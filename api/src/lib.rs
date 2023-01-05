use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
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
    let vec = buffer.split("\n\n").collect::<Vec<&str>>();
    let body = vec.get(1).ok_or("missing body")?;
    Ok(body.to_string())
}
pub mod get {
    use super::*;
    use pea_core::types;
    use std::error::Error;
    pub async fn index(api: &str) -> Result<types::api::Index, Box<dyn Error>> {
        Ok(request(api, Method::GET, "/", None).await?)
    }
    pub async fn info(api: &str) -> Result<types::api::Info, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/info", None).await?)?)
    }
    pub async fn sync(api: &str) -> Result<types::api::Sync, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/sync", None).await?)?)
    }
    pub async fn height(api: &str) -> Result<types::api::Height, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/height", None).await?)?)
    }
    pub async fn balance(api: &str, address: &str) -> Result<types::api::Amount, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/balance/{}", address), None).await?)?)
    }
    pub async fn balance_staked(api: &str, address: &str) -> Result<types::api::Amount, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::GET, &format!("/balance_staked/{}", address), None).await?,
        )?)
    }
    pub async fn hash(api: &str, height: &usize) -> Result<types::api::Hash, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/hash/{}", height), None).await?)?)
    }
    pub async fn dynamic(api: &str) -> Result<types::api::Dynamic, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/dynamic", None).await?)?)
    }
    pub async fn trusted(api: &str) -> Result<types::api::Trusted, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/trusted", None).await?)?)
    }
    pub async fn options(api: &str) -> Result<types::api::Options, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/options", None).await?)?)
    }
    pub async fn block(api: &str, hash: &str) -> Result<types::api::Block, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/block/{}", hash), None).await?)?)
    }
    pub async fn latest_block(api: &str) -> Result<types::api::Block, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, "/block/latest", None).await?)?)
    }
    pub async fn transaction(api: &str, hash: &str) -> Result<types::api::Transaction, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::GET, &format!("/transaction/{}", hash), None).await?,
        )?)
    }
    pub async fn stake(api: &str, hash: &str) -> Result<types::api::Stake, Box<dyn Error>> {
        Ok(serde_json::from_str(&request(api, Method::GET, &format!("/stake/{}", hash), None).await?)?)
    }
}
pub mod post {
    use super::*;
    use pea_stake::StakeB;
    use pea_transaction::TransactionB;
    use std::error::Error;
    pub async fn transaction(api: &str, transaction_b: &TransactionB) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::POST, "/transaction", Some(&hex::encode(bincode::serialize(transaction_b)?))).await?,
        )?)
    }
    pub async fn stake(api: &str, stake_b: &StakeB) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::from_str(
            &request(api, Method::POST, "/stake", Some(&hex::encode(bincode::serialize(stake_b)?))).await?,
        )?)
    }
}
