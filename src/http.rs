use super::{
    block::Block, blockchain::Stakers, p2p::MyBehaviour, stake::Stake, transaction::Transaction,
    validator::Synchronizer, wallet::address,
};
use crate::block::BlockMetadata;
use serde::{Deserialize, Serialize};
pub mod regex {
    use lazy_static::lazy_static;
    use regex::Regex;
    lazy_static! {
        pub static ref GET: Regex = Regex::new(r"^GET [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
        pub static ref POST: Regex = Regex::new(r"^POST [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
        pub static ref INDEX: Regex = Regex::new(r" / ").unwrap();
        pub static ref JSON: Regex = Regex::new(r" /json ").unwrap();
        pub static ref BALANCE: Regex = Regex::new(r" /balance/0[xX][0-9A-Fa-f]* ").unwrap();
        pub static ref BALANCE_STAKED: Regex =
            Regex::new(r" /balance_staked/0[xX][0-9A-Fa-f]* ").unwrap();
        pub static ref HEIGHT: Regex = Regex::new(r" /height ").unwrap();
        pub static ref HASH_BY_HEIGHT: Regex = Regex::new(r" /hash/[0-9]+ ").unwrap();
        pub static ref TRANSACTION: Regex = Regex::new(r" /transaction ").unwrap();
        pub static ref STAKE: Regex = Regex::new(r" /stake ").unwrap();
    }
}
pub fn format_height(height: usize) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
        height
    )
}
pub fn format_hash_by_height(hash: &str) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
        hash
    )
}
pub fn format_balance(balance: u64) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
        balance
    )
}
pub fn format_status(status: usize) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
        status
    )
}
pub fn format_404() -> String {
    format!(
        "\
HTTP/1.1 404 NOT FOUND"
    )
}
pub fn format_400() -> String {
    format!(
        "\
HTTP/1.1 400 BAD REQUEST"
    )
}
pub fn format_index(behaviour: &MyBehaviour) -> String {
    format!(
        "\
HTTP/1.1 200 OK

Validator {} {}/tree/{}

 public_key: {}

 balance: {}

 staked_balance: {}

 height: {}

 heartbeats: {}

 lag: {:?}

 {:?}

 queue: {:?}

 latest_hashes: {:?}

 pending_transactions: {:?}

 pending_stakes: {:?}

 pending_blocks: {:?}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
        env!("GIT_HASH"),
        address::encode(&behaviour.validator.keypair.public.as_bytes()),
        match behaviour.validator.blockchain.get_balance(
            &behaviour.validator.db,
            &behaviour.validator.keypair.public.as_bytes()
        ) {
            Ok(balance) => balance.to_string(),
            Err(_) => "Err".to_string(),
        },
        match behaviour.validator.blockchain.get_staked_balance(
            &behaviour.validator.db,
            &behaviour.validator.keypair.public.as_bytes()
        ) {
            Ok(balance) => balance.to_string(),
            Err(_) => "Err".to_string(),
        },
        behaviour.validator.blockchain.latest_height(),
        behaviour.validator.heartbeats,
        behaviour.validator.lag,
        behaviour.validator.synchronizer,
        behaviour
            .validator
            .blockchain
            .stakers
            .queue
            .iter()
            .map(|&x| (address::encode(&x.0), x.1, x.2))
            .collect::<Vec<(String, u64, usize)>>(),
        behaviour
            .validator
            .blockchain
            .hashes
            .iter()
            .rev()
            .take(3)
            .map(|&x| hex::encode(x))
            .collect::<Vec<String>>(),
        behaviour
            .validator
            .blockchain
            .pending_transactions
            .iter()
            .map(|x| hex::encode(x.hash()))
            .collect::<Vec<String>>(),
        behaviour
            .validator
            .blockchain
            .pending_stakes
            .iter()
            .map(|x| hex::encode(x.hash()))
            .collect::<Vec<String>>(),
        behaviour
            .validator
            .blockchain
            .pending_blocks
            .iter()
            .map(|x| hex::encode(BlockMetadata::from(x).hash()))
            .collect::<Vec<String>>(),
    )
}
#[derive(Serialize, Deserialize)]
struct Data {
    public_key: [u8; 32],
    balance: u64,
    staked_balance: u64,
    height: usize,
    heartbeats: usize,
    lag: [f64; 3],
    synchronizer: Synchronizer,
    stakers: Stakers,
    latest_hashes: Vec<[u8; 32]>,
    pending_transactions: Vec<Transaction>,
    pending_stakes: Vec<Stake>,
    pending_blocks: Vec<Block>,
    latest_block: Block,
}
pub fn format_json(behaviour: &MyBehaviour) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
        serde_json::to_string(&Data {
            public_key: *behaviour.validator.keypair.public.as_bytes(),
            balance: behaviour
                .validator
                .blockchain
                .get_balance(
                    &behaviour.validator.db,
                    &behaviour.validator.keypair.public.as_bytes()
                )
                .unwrap(),
            staked_balance: behaviour
                .validator
                .blockchain
                .get_staked_balance(
                    &behaviour.validator.db,
                    &behaviour.validator.keypair.public.as_bytes()
                )
                .unwrap(),
            height: behaviour.validator.blockchain.latest_height(),
            heartbeats: behaviour.validator.heartbeats,
            lag: behaviour.validator.lag,
            synchronizer: behaviour.validator.synchronizer,
            stakers: behaviour.validator.blockchain.stakers.clone(),
            latest_hashes: behaviour
                .validator
                .blockchain
                .hashes
                .iter()
                .rev()
                .take(10)
                .cloned()
                .collect(),
            pending_transactions: behaviour.validator.blockchain.pending_transactions.clone(),
            pending_stakes: behaviour.validator.blockchain.pending_stakes.clone(),
            pending_blocks: behaviour.validator.blockchain.pending_blocks.clone(),
            latest_block: behaviour.validator.blockchain.latest_block.clone()
        })
        .unwrap()
    )
}
