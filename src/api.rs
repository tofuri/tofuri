pub mod get {
    use crate::types;
    use serde::{Deserialize, Serialize};
    use std::error::Error;
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
    pub async fn height(api: &str) -> Result<types::Height, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/height", api))
            .await?
            .json()
            .await?)
    }
    pub async fn balance(api: &str, address: &str) -> Result<types::Amount, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance/{}", api, address))
            .await?
            .json()
            .await?)
    }
    pub async fn balance_staked(api: &str, address: &str) -> Result<types::Amount, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance_staked/{}", api, address))
            .await?
            .json()
            .await?)
    }
    pub async fn index(api: &str) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::get(api).await?.text().await?)
    }
    pub async fn block(api: &str, hash: &str) -> Result<Block, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/block/{}", api, hash))
            .await?
            .json()
            .await?)
    }
    pub async fn transaction(api: &str, hash: &str) -> Result<Transaction, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/transaction/{}", api, hash))
            .await?
            .json()
            .await?)
    }
    pub async fn stake(api: &str, hash: &str) -> Result<Stake, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/stake/{}", api, hash))
            .await?
            .json()
            .await?)
    }
    pub async fn hash(api: &str, height: &usize) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/hash/{}", api, height))
            .await?
            .json()
            .await?)
    }
}
pub mod post {
    use crate::{
        stake::{CompressedStake, Stake},
        transaction::{CompressedTransaction, Transaction},
    };
    use std::error::Error;
    pub async fn transaction(
        api: &str,
        transaction: &Transaction,
    ) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/transaction", api))
            .body(hex::encode(bincode::serialize(
                &CompressedTransaction::from(transaction),
            )?))
            .send()
            .await?
            .json()
            .await?)
    }
    pub async fn stake(api: &str, stake: &Stake) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/stake", api))
            .body(hex::encode(bincode::serialize(&CompressedStake::from(
                stake,
            ))?))
            .send()
            .await?
            .json()
            .await?)
    }
}
