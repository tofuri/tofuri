pub mod get {
    use pea_core::types;
    use std::error::Error;
    pub async fn info(api: &str) -> Result<types::api::Data, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/info", api)).await?.json().await?)
    }
    pub async fn sync(api: &str) -> Result<types::api::Sync, Box<dyn Error>> {
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
    pub async fn block(api: &str, hash: &str) -> Result<types::api::Block, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/block/{}", api, hash)).await?.json().await?)
    }
    pub async fn latest_block(api: &str) -> Result<types::api::Block, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/block/latest", api)).await?.json().await?)
    }
    pub async fn transaction(api: &str, hash: &str) -> Result<types::api::Transaction, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/transaction/{}", api, hash)).await?.json().await?)
    }
    pub async fn stake(api: &str, hash: &str) -> Result<types::api::Stake, Box<dyn Error>> {
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
