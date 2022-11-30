pub mod get {
    use serde::{Deserialize, Serialize};
    use std::error::Error;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Sync {
        pub sync: String,
        pub last: String,
        pub height: usize,
        pub peers: usize,
        pub index: usize,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct State {
        pub balance: u128,
        pub balance_staked: u128,
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
        pub pending: usize,
        pub ban_offline: usize,
        pub time_delta: u32,
        pub max_established: Option<u32>,
        pub tps: f64,
        pub bind_api: String,
        pub host: String,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Data {
        pub public_key: String,
        pub tree_size: usize,
        pub heartbeats: usize,
        pub lag: f64,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Block {
        pub hash: String,
        pub previous_hash: String,
        pub timestamp: u32,
        pub public_key: String,
        pub signature: String,
        pub transactions: Vec<String>,
        pub stakes: Vec<String>,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Transaction {
        pub hash: String,
        pub public_key_input: String,
        pub public_key_output: String,
        pub amount: u128,
        pub fee: u128,
        pub timestamp: u32,
        pub signature: String,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Stake {
        pub hash: String,
        pub public_key: String,
        pub amount: u128,
        pub deposit: bool,
        pub fee: u128,
        pub timestamp: u32,
        pub signature: String,
    }
    pub async fn info(api: &str) -> Result<Data, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/info", api)).await?.json().await?)
    }
    pub async fn sync(api: &str) -> Result<Sync, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/sync", api)).await?.json().await?)
    }
    pub async fn height(api: &str) -> Result<usize, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/height", api)).await?.json().await?)
    }
    pub async fn balance(api: &str, address: &str) -> Result<u128, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance/{}", api, address)).await?.json().await?)
    }
    pub async fn balance_staked(api: &str, address: &str) -> Result<u128, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance_staked/{}", api, address)).await?.json().await?)
    }
    pub async fn index(api: &str) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::get(api).await?.text().await?)
    }
    pub async fn block(api: &str, hash: &str) -> Result<Block, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/block/{}", api, hash)).await?.json().await?)
    }
    pub async fn latest_block(api: &str) -> Result<Block, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/block/latest", api)).await?.json().await?)
    }
    pub async fn transaction(api: &str, hash: &str) -> Result<Transaction, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/transaction/{}", api, hash)).await?.json().await?)
    }
    pub async fn stake(api: &str, hash: &str) -> Result<Stake, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/stake/{}", api, hash)).await?.json().await?)
    }
    pub async fn hash(api: &str, height: &usize) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/hash/{}", api, height)).await?.json().await?)
    }
}
pub mod post {
    use pea_stake::Stake;
    use pea_transaction::Transaction;
    use std::error::Error;
    pub async fn transaction(api: &str, transaction: &Transaction) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/transaction", api))
            .body(hex::encode(bincode::serialize(transaction)?))
            .send()
            .await?
            .json()
            .await?)
    }
    pub async fn stake(api: &str, stake: &Stake) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/stake", api))
            .body(hex::encode(bincode::serialize(stake)?))
            .send()
            .await?
            .json()
            .await?)
    }
}
