use crate::node::Node;
use colored::*;
use lazy_static::lazy_static;
use log::{error, info};
use pea_address as address;
use pea_api::get;
use pea_db as db;
use pea_stake::{self as stake, Stake};
use pea_transaction::{self as transaction, Transaction};
use regex::Regex;
use std::{error::Error, io::BufRead};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
lazy_static! {
    static ref GET: Regex = Regex::new(r"^GET [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
    static ref POST: Regex = Regex::new(r"^POST [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
    static ref INDEX: Regex = Regex::new(r" / ").unwrap();
    static ref JSON: Regex = Regex::new(r" /json ").unwrap();
    static ref BALANCE: Regex = Regex::new(r" /balance/0[xX][0-9A-Fa-f]* ").unwrap();
    static ref BALANCE_STAKED: Regex = Regex::new(r" /balance_staked/0[xX][0-9A-Fa-f]* ").unwrap();
    static ref HEIGHT: Regex = Regex::new(r" /height ").unwrap();
    static ref BLOCK_LATEST: Regex = Regex::new(r" /block/latest ").unwrap();
    static ref HASH_BY_HEIGHT: Regex = Regex::new(r" /hash/[0-9]+ ").unwrap();
    static ref BLOCK_BY_HASH: Regex = Regex::new(r" /block/[0-9A-Fa-f]* ").unwrap();
    static ref TRANSACTION_BY_HASH: Regex = Regex::new(r" /transaction/[0-9A-Fa-f]* ").unwrap();
    static ref STAKE_BY_HASH: Regex = Regex::new(r" /stake/[0-9A-Fa-f]* ").unwrap();
    static ref TRANSACTION: Regex = Regex::new(r" /transaction ").unwrap();
    static ref TRANSACTION_SERIALIZED: usize = hex::encode({
        let transaction = Transaction::new([0; 32], 0, 0);
        bincode::serialize(&transaction::Compressed {
            public_key_input: transaction.public_key_input,
            public_key_output: transaction.public_key_output,
            amount: pea_amount::to_bytes(&transaction.amount),
            fee: pea_amount::to_bytes(&transaction.fee),
            timestamp: transaction.timestamp,
            signature: transaction.signature,
        })
        .unwrap()
    })
    .len();
    static ref STAKE: Regex = Regex::new(r" /stake ").unwrap();
    static ref STAKE_SERIALIZED: usize = hex::encode({
        let stake = Stake::new(false, 0, 0);
        bincode::serialize(&stake::Compressed {
            public_key: stake.public_key,
            amount: pea_amount::to_bytes(&stake.amount),
            fee: pea_amount::to_bytes(&stake.fee),
            deposit: stake.deposit,
            timestamp: stake.timestamp,
            signature: stake.signature,
        })
        .unwrap()
    })
    .len();
    static ref PEERS: Regex = Regex::new(r" /peers ").unwrap();
}
pub async fn next(listener: &tokio::net::TcpListener) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
    Ok(listener.accept().await?.0)
}
pub async fn handler(mut stream: tokio::net::TcpStream, node: &mut Node) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await?;
    let first = buffer.lines().next().ok_or("http request first line")??;
    info!("{} {} {}", "API".cyan(), stream.peer_addr()?.to_string().magenta(), first);
    if GET.is_match(&first) {
        handler_get(&mut stream, node, &first).await?;
    } else if POST.is_match(&first) {
        handler_post(&mut stream, node, &first, &buffer).await?;
    } else {
        handler_404(&mut stream).await?;
    };
    stream.flush().await?;
    Ok(())
}
async fn handler_get(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str) -> Result<(), Box<dyn Error>> {
    if INDEX.is_match(first) {
        handler_get_index(stream).await?;
    } else if JSON.is_match(first) {
        handler_get_json(stream, node).await?;
    } else if BALANCE.is_match(first) {
        handler_get_json_balance(stream, node, first).await?;
    } else if BALANCE_STAKED.is_match(first) {
        handler_get_json_balance_staked(stream, node, first).await?;
    } else if HEIGHT.is_match(first) {
        handler_get_json_height(stream, node).await?;
    } else if BLOCK_LATEST.is_match(first) {
        handler_get_json_block_latest(stream, node).await?;
    } else if HASH_BY_HEIGHT.is_match(first) {
        handler_get_json_hash_by_height(stream, node, first).await?;
    } else if BLOCK_BY_HASH.is_match(first) {
        handler_get_json_block_by_hash(stream, node, first).await?;
    } else if TRANSACTION_BY_HASH.is_match(first) {
        handler_get_json_transaction_by_hash(stream, node, first).await?;
    } else if STAKE_BY_HASH.is_match(first) {
        handler_get_json_stake_by_hash(stream, node, first).await?;
    } else if PEERS.is_match(first) {
        handler_get_json_peers(stream, node).await?;
    } else {
        handler_404(stream).await?;
    };
    Ok(())
}
async fn handler_post(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str, buffer: &[u8; 1024]) -> Result<(), Box<dyn Error>> {
    if TRANSACTION.is_match(first) {
        handler_post_json_transaction(stream, node, buffer).await?;
    } else if STAKE.is_match(first) {
        handler_post_json_stake(stream, node, buffer).await?;
    } else {
        handler_404(stream).await?;
    };
    Ok(())
}
async fn handler_get_index(stream: &mut tokio::net::TcpStream) -> Result<(), Box<dyn Error>> {
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *

{} {}
{}/tree/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_REPOSITORY"),
                env!("GIT_HASH"),
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json(stream: &mut tokio::net::TcpStream, node: &mut Node) -> Result<(), Box<dyn Error>> {
    let states = &node.blockchain.states;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&get::Data {
                    public_key: node.blockchain.key.public(),
                    height: node.blockchain.height(),
                    tree_size: node.blockchain.tree.size(),
                    heartbeats: node.heartbeats,
                    gossipsub_peers: node.swarm.behaviour().gossipsub.all_peers().count(),
                    states: get::States {
                        dynamic: get::State {
                            balance: states.dynamic.balance(&node.blockchain.key.public_key_bytes()),
                            balance_staked: states.dynamic.balance_staked(&node.blockchain.key.public_key_bytes()),
                            hashes: states.dynamic.hashes.len(),
                            latest_hashes: states.dynamic.hashes.iter().rev().take(16).map(hex::encode).collect(),
                            stakers: states.dynamic.stakers.iter().map(address::public::encode).collect(),
                        },
                        trusted: get::State {
                            balance: states.trusted.balance(&node.blockchain.key.public_key_bytes()),
                            balance_staked: states.trusted.balance_staked(&node.blockchain.key.public_key_bytes()),
                            stakers: states.trusted.stakers.iter().map(address::public::encode).collect(),
                            hashes: states.trusted.hashes.len(),
                            latest_hashes: states.trusted.hashes.iter().rev().take(16).map(hex::encode).collect(),
                        },
                    },
                    lag: node.lag,
                    pending_transactions: node.blockchain.pending_transactions.iter().map(|x| hex::encode(x.hash())).collect(),
                    pending_stakes: node.blockchain.pending_stakes.iter().map(|x| hex::encode(x.hash())).collect(),
                    pending_blocks: node.blockchain.pending_blocks.iter().map(|x| hex::encode(x.hash())).collect(),
                    sync_index: node.blockchain.sync.index_0,
                    syncing: node.blockchain.sync.syncing,
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_balance(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str) -> Result<(), Box<dyn Error>> {
    let public_key = address::public::decode(BALANCE.find(first).ok_or("GET BALANCE 1")?.as_str().trim().get(9..).ok_or("GET BALANCE 2")?)?;
    let balance = node.blockchain.states.dynamic.balance(&public_key);
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&balance)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_balance_staked(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str) -> Result<(), Box<dyn Error>> {
    let public_key = address::public::decode(
        BALANCE_STAKED
            .find(first)
            .ok_or("GET BALANCE_STAKED 1")?
            .as_str()
            .trim()
            .get(16..)
            .ok_or("GET BALANCE_STAKED 2")?,
    )?;
    let balance = node.blockchain.states.dynamic.balance_staked(&public_key);
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&balance)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_height(stream: &mut tokio::net::TcpStream, node: &mut Node) -> Result<(), Box<dyn Error>> {
    let height = node.blockchain.height();
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&height)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_block_latest(stream: &mut tokio::net::TcpStream, node: &mut Node) -> Result<(), Box<dyn Error>> {
    let block = node.blockchain.latest_block();
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&get::Block {
                    hash: hex::encode(block.hash()),
                    previous_hash: hex::encode(block.previous_hash),
                    timestamp: block.timestamp,
                    public_key: address::public::encode(&block.public_key),
                    signature: hex::encode(block.signature),
                    transactions: block.transactions.iter().map(|x| hex::encode(x.hash())).collect(),
                    stakes: block.stakes.iter().map(|x| hex::encode(x.hash())).collect(),
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_hash_by_height(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str) -> Result<(), Box<dyn Error>> {
    let height = HASH_BY_HEIGHT
        .find(first)
        .ok_or("GET HASH_BY_HEIGHT 1")?
        .as_str()
        .trim()
        .get(6..)
        .ok_or("GET HASH_BY_HEIGHT 2")?
        .parse::<usize>()?;
    let states = &node.blockchain.states;
    let hashes_trusted = &states.trusted.hashes;
    let hashes_dynamic = &states.dynamic.hashes;
    if height >= hashes_trusted.len() + hashes_dynamic.len() {
        return Err("GET HASH_BY_HEIGHT 3".into());
    }
    let hash = if height < hashes_trusted.len() {
        hashes_trusted[height]
    } else {
        hashes_dynamic[height - hashes_trusted.len()]
    };
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&hex::encode(hash))?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_block_by_hash(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str) -> Result<(), Box<dyn Error>> {
    let hash = hex::decode(
        BLOCK_BY_HASH
            .find(first)
            .ok_or("GET BLOCK_BY_HASH 1")?
            .as_str()
            .trim()
            .get(7..)
            .ok_or("GET BLOCK_BY_HASH 2")?,
    )?;
    let block = db::block::get(&node.blockchain.db, &hash)?;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&get::Block {
                    hash: hex::encode(block.hash()),
                    previous_hash: hex::encode(block.previous_hash),
                    timestamp: block.timestamp,
                    public_key: address::public::encode(&block.public_key),
                    signature: hex::encode(block.signature),
                    transactions: block.transactions.iter().map(|x| hex::encode(x.hash())).collect(),
                    stakes: block.stakes.iter().map(|x| hex::encode(x.hash())).collect(),
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_transaction_by_hash(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str) -> Result<(), Box<dyn Error>> {
    let hash = hex::decode(
        TRANSACTION_BY_HASH
            .find(first)
            .ok_or("GET TRANSACTION_BY_HASH 1")?
            .as_str()
            .trim()
            .get(13..)
            .ok_or("GET TRANSACTION_BY_HASH 2")?,
    )?;
    let transaction = db::transaction::get(&node.blockchain.db, &hash)?;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&get::Transaction {
                    hash: hex::encode(transaction.hash()),
                    public_key_input: address::public::encode(&transaction.public_key_input),
                    public_key_output: address::public::encode(&transaction.public_key_output),
                    amount: transaction.amount,
                    fee: transaction.fee,
                    timestamp: transaction.timestamp,
                    signature: hex::encode(transaction.signature)
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_stake_by_hash(stream: &mut tokio::net::TcpStream, node: &mut Node, first: &str) -> Result<(), Box<dyn Error>> {
    let hash = hex::decode(
        STAKE_BY_HASH
            .find(first)
            .ok_or("GET STAKE_BY_HASH 1")?
            .as_str()
            .trim()
            .get(7..)
            .ok_or("GET STAKE_BY_HASH 2")?,
    )?;
    let stake = db::stake::get(&node.blockchain.db, &hash)?;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&get::Stake {
                    hash: hex::encode(stake.hash()),
                    public_key: address::public::encode(&stake.public_key),
                    amount: stake.amount,
                    deposit: stake.deposit,
                    fee: stake.fee,
                    timestamp: stake.timestamp,
                    signature: hex::encode(stake.signature)
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_peers(stream: &mut tokio::net::TcpStream, node: &mut Node) -> Result<(), Box<dyn Error>> {
    let peers = db::peer::get_all(&node.blockchain.db);
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&peers)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_post_json_transaction(stream: &mut tokio::net::TcpStream, node: &mut Node, buffer: &[u8; 1024]) -> Result<(), Box<dyn Error>> {
    let compressed: transaction::Compressed = bincode::deserialize(&hex::decode(
        buffer
            .lines()
            .nth(5)
            .ok_or("POST TRANSACTION 1")??
            .get(0..*TRANSACTION_SERIALIZED)
            .ok_or("POST TRANSACTION 2")?,
    )?)?;
    let status = match node.blockchain.pending_transactions_push(Transaction {
        public_key_input: compressed.public_key_input,
        public_key_output: compressed.public_key_output,
        amount: pea_amount::from_bytes(&compressed.amount),
        fee: pea_amount::from_bytes(&compressed.fee),
        timestamp: compressed.timestamp,
        signature: compressed.signature,
    }) {
        Ok(()) => "success".to_string(),
        Err(err) => {
            error!("{}", err);
            err.to_string()
        }
    };
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&status)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_post_json_stake(stream: &mut tokio::net::TcpStream, node: &mut Node, buffer: &[u8; 1024]) -> Result<(), Box<dyn Error>> {
    let compressed: stake::Compressed = bincode::deserialize(&hex::decode(
        buffer.lines().nth(5).ok_or("POST STAKE 1")??.get(0..*STAKE_SERIALIZED).ok_or("POST STAKE 2")?,
    )?)?;
    let status = match node.blockchain.pending_stakes_push(Stake {
        public_key: compressed.public_key,
        amount: pea_amount::from_bytes(&compressed.amount),
        fee: pea_amount::from_bytes(&compressed.fee),
        deposit: compressed.deposit,
        timestamp: compressed.timestamp,
        signature: compressed.signature,
    }) {
        Ok(()) => "success".to_string(),
        Err(err) => {
            error!("{}", err);
            err.to_string()
        }
    };
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&status)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_404(stream: &mut tokio::net::TcpStream) -> Result<(), Box<dyn Error>> {
    stream.write_all("HTTP/1.1 404 NOT FOUND".as_bytes()).await?;
    Ok(())
}
