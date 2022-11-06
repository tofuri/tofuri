pub mod get {
    use pea_core::types;
    use serde::{Deserialize, Serialize};
    use std::error::Error;
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Data {
        pub public_key: String,
        pub height: types::Height,
        pub tree_size: usize,
        pub heartbeats: types::Heartbeats,
        pub lag: f64,
        pub gossipsub_peers: usize,
        pub states: States,
        pub pending_transactions: Vec<String>,
        pub pending_stakes: Vec<String>,
        pub pending_blocks: Vec<String>,
        pub sync_index: usize,
        pub syncing: bool,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct States {
        pub dynamic: State,
        pub trusted: State,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct State {
        pub balance: types::Amount,
        pub balance_staked: types::Amount,
        pub hashes: usize,
        pub latest_hashes: Vec<String>,
        pub stakers: Vec<String>,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Block {
        pub previous_hash: String,
        pub timestamp: types::Timestamp,
        pub public_key: String,
        pub signature: String,
        pub transactions: Vec<String>,
        pub stakes: Vec<String>,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Transaction {
        pub public_key_input: String,
        pub public_key_output: String,
        pub amount: types::Amount,
        pub fee: types::Amount,
        pub timestamp: types::Timestamp,
        pub signature: String,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Stake {
        pub public_key: String,
        pub amount: types::Amount,
        pub deposit: bool,
        pub fee: types::Amount,
        pub timestamp: types::Timestamp,
        pub signature: String,
    }
    pub async fn data(api: &str) -> Result<Data, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/json", api)).await?.json().await?)
    }
    pub async fn height(api: &str) -> Result<types::Height, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/height", api)).await?.json().await?)
    }
    pub async fn balance(api: &str, address: &str) -> Result<types::Amount, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance/{}", api, address)).await?.json().await?)
    }
    pub async fn balance_staked(api: &str, address: &str) -> Result<types::Amount, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance_staked/{}", api, address)).await?.json().await?)
    }
    pub async fn index(api: &str) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::get(api).await?.text().await?)
    }
    pub async fn block(api: &str, hash: &str) -> Result<Block, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/block/{}", api, hash)).await?.json().await?)
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
    use pea_core::{
        stake::{self, Stake},
        transaction::{self, Transaction},
    };
    use std::error::Error;
    pub async fn transaction(api: &str, transaction: &Transaction) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/transaction", api))
            .body(hex::encode(bincode::serialize(&transaction::Compressed {
                public_key_input: transaction.public_key_input,
                public_key_output: transaction.public_key_output,
                amount: pea_amount::to_bytes(&transaction.amount),
                fee: pea_amount::to_bytes(&transaction.fee),
                timestamp: transaction.timestamp,
                signature: transaction.signature,
            })?))
            .send()
            .await?
            .json()
            .await?)
    }
    pub async fn stake(api: &str, stake: &Stake) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/stake", api))
            .body(hex::encode(bincode::serialize(&stake::Compressed {
                public_key: stake.public_key,
                amount: pea_amount::to_bytes(&stake.amount),
                fee: pea_amount::to_bytes(&stake.fee),
                deposit: stake.deposit,
                timestamp: stake.timestamp,
                signature: stake.signature,
            })?))
            .send()
            .await?
            .json()
            .await?)
    }
}
