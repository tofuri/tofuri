pub mod get {
    use pea_core::types;
    use std::error::Error;
    pub async fn index(api: &str) -> Result<types::api::Index, Box<dyn Error>> {
        Ok(reqwest::get(api).await?.json().await?)
    }
    pub async fn info(api: &str) -> Result<types::api::Info, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/info", api)).await?.json().await?)
    }
    pub async fn sync(api: &str) -> Result<types::api::Sync, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/sync", api)).await?.json().await?)
    }
    pub async fn height(api: &str) -> Result<types::api::Height, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/height", api)).await?.json().await?)
    }
    pub async fn balance(api: &str, address: &str) -> Result<types::api::Amount, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance/{}", api, address)).await?.json().await?)
    }
    pub async fn balance_staked(api: &str, address: &str) -> Result<types::api::Amount, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/balance_staked/{}", api, address)).await?.json().await?)
    }
    pub async fn hash(api: &str, height: &usize) -> Result<types::api::Hash, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/hash/{}", api, height)).await?.json().await?)
    }
    pub async fn dynamic(api: &str) -> Result<types::api::Dynamic, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/dynamic", api)).await?.json().await?)
    }
    pub async fn trusted(api: &str) -> Result<types::api::Trusted, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/trusted", api)).await?.json().await?)
    }
    pub async fn options(api: &str) -> Result<types::api::Options, Box<dyn Error>> {
        Ok(reqwest::get(format!("{}/options", api)).await?.json().await?)
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
}
pub mod post {
    use pea_stake::StakeC;
    use pea_transaction::TransactionB;
    use std::error::Error;
    pub async fn transaction(api: &str, transaction_b: &TransactionB) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/transaction", api))
            .body(hex::encode(bincode::serialize(transaction_b)?))
            .send()
            .await?
            .json()
            .await?)
    }
    pub async fn stake(api: &str, stake: &StakeC) -> Result<String, Box<dyn Error>> {
        Ok(reqwest::Client::new()
            .post(format!("{}/stake", api))
            .body(hex::encode(bincode::serialize(stake)?))
            .send()
            .await?
            .json()
            .await?)
    }
}
