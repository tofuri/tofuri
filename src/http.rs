use crate::{
    address,
    block::Block,
    p2p::MyBehaviour,
    print,
    stake::{CompressedStake, Stake},
    transaction::{CompressedTransaction, Transaction},
    types,
};
use lazy_static::lazy_static;
use libp2p::Swarm;
use log::error;
use regex::Regex;
use serde::Serialize;
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
    static ref HASH_BY_HEIGHT: Regex = Regex::new(r" /hash/[0-9]+ ").unwrap();
    static ref BLOCK_BY_HASH: Regex = Regex::new(r" /block/[0-9A-Fa-f]* ").unwrap();
    static ref TRANSACTION_BY_HASH: Regex = Regex::new(r" /transaction/[0-9A-Fa-f]* ").unwrap();
    static ref STAKE_BY_HASH: Regex = Regex::new(r" /stake/[0-9A-Fa-f]* ").unwrap();
    static ref TRANSACTION: Regex = Regex::new(r" /transaction ").unwrap();
    static ref TRANSACTION_SERIALIZED: usize = hex::encode(
        bincode::serialize(&CompressedTransaction::from(&Transaction::new(
            [0; 32], 0, 0
        )))
        .unwrap()
    )
    .len();
    static ref STAKE: Regex = Regex::new(r" /stake ").unwrap();
    static ref STAKE_SERIALIZED: usize =
        hex::encode(bincode::serialize(&CompressedStake::from(&Stake::new(false, 0, 0))).unwrap())
            .len();
}
pub async fn next(
    listener: &tokio::net::TcpListener,
) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
    Ok(listener.accept().await?.0)
}
pub async fn handler(
    mut stream: tokio::net::TcpStream,
    swarm: &mut Swarm<MyBehaviour>,
) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await?;
    let first = buffer.lines().next().ok_or("http request first line")??;
    print::http_handler(&first);
    if GET.is_match(&first) {
        handler_get(&mut stream, swarm, &first).await?;
    } else if POST.is_match(&first) {
        handler_post(&mut stream, swarm, &first, &buffer).await?;
    } else {
        handler_404(&mut stream).await?;
    };
    stream.flush().await?;
    Ok(())
}
async fn handler_get(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
    first: &str,
) -> Result<(), Box<dyn Error>> {
    if INDEX.is_match(first) {
        handler_get_index(stream).await?;
    } else if JSON.is_match(first) {
        handler_get_json(stream, swarm).await?;
    } else if BALANCE.is_match(first) {
        handler_get_json_balance(stream, swarm, first).await?;
    } else if BALANCE_STAKED.is_match(first) {
        handler_get_json_balance_staked(stream, swarm, first).await?;
    } else if HEIGHT.is_match(first) {
        handler_get_json_height(stream, swarm).await?;
    } else if HASH_BY_HEIGHT.is_match(first) {
        handler_get_json_hash_by_height(stream, swarm, first).await?;
    } else if BLOCK_BY_HASH.is_match(first) {
        handler_get_json_block_by_hash(stream, swarm, first).await?;
    } else if TRANSACTION_BY_HASH.is_match(first) {
        handler_get_json_transaction_by_hash(stream, swarm, first).await?;
    } else if STAKE_BY_HASH.is_match(first) {
        handler_get_json_stake_by_hash(stream, swarm, first).await?;
    } else {
        handler_404(stream).await?;
    };
    Ok(())
}
async fn handler_post(
    stream: &mut tokio::net::TcpStream,
    swarm: &mut Swarm<MyBehaviour>,
    first: &str,
    buffer: &[u8; 1024],
) -> Result<(), Box<dyn Error>> {
    if TRANSACTION.is_match(first) {
        handler_post_json_transaction(stream, swarm, buffer).await?;
    } else if STAKE.is_match(first) {
        handler_post_json_stake(stream, swarm, buffer).await?;
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
async fn handler_get_json(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
) -> Result<(), Box<dyn Error>> {
    #[derive(Serialize)]
    struct Data {
        public_key: String,
        height: types::Height,
        tree_size: usize,
        heartbeats: types::Heartbeats,
        lag: f64,
        gossipsub_peers: usize,
        states: States,
        pending_transactions: Vec<String>,
        pending_stakes: Vec<String>,
        pending_blocks: Vec<String>,
        sync_index: usize,
        syncing: bool,
    }
    #[derive(Serialize)]
    struct States {
        dynamic: State,
        trusted: State,
    }
    #[derive(Serialize)]
    struct State {
        balance: types::Amount,
        balance_staked: types::Amount,
        hashes: usize,
        latest_hashes: Vec<String>,
        stakers: Vec<String>,
    }
    let behaviour = swarm.behaviour();
    let states = &behaviour.blockchain.states;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
                serde_json::to_string(&Data {
                    public_key: address::encode(behaviour.blockchain.keypair.public.as_bytes()),
                    height: behaviour.blockchain.height(),
                    tree_size: behaviour.blockchain.tree.size(),
                    heartbeats: behaviour.heartbeats,
                    gossipsub_peers: behaviour.gossipsub.all_peers().count(),
                    states: States {
                        dynamic: State {
                            balance: states
                                .dynamic
                                .balance(behaviour.blockchain.keypair.public.as_bytes()),
                            balance_staked: states
                                .dynamic
                                .balance_staked(behaviour.blockchain.keypair.public.as_bytes()),
                            hashes: states.dynamic.hashes.len(),
                            latest_hashes: states
                                .dynamic
                                .hashes
                                .iter()
                                .rev()
                                .take(16)
                                .map(hex::encode)
                                .collect(),
                            stakers: states.dynamic.stakers.iter().map(address::encode).collect(),
                        },
                        trusted: State {
                            balance: states
                                .trusted
                                .balance(behaviour.blockchain.keypair.public.as_bytes()),
                            balance_staked: states
                                .trusted
                                .balance_staked(behaviour.blockchain.keypair.public.as_bytes()),
                            stakers: states.trusted.stakers.iter().map(address::encode).collect(),
                            hashes: states.trusted.hashes.len(),
                            latest_hashes: states
                                .trusted
                                .hashes
                                .iter()
                                .rev()
                                .take(16)
                                .map(hex::encode)
                                .collect(),
                        },
                    },
                    lag: behaviour.lag,
                    pending_transactions: behaviour
                        .blockchain
                        .pending_transactions
                        .iter()
                        .map(|x| hex::encode(x.hash()))
                        .collect(),
                    pending_stakes: behaviour
                        .blockchain
                        .pending_stakes
                        .iter()
                        .map(|x| hex::encode(x.hash()))
                        .collect(),
                    pending_blocks: behaviour
                        .blockchain
                        .pending_blocks
                        .iter()
                        .map(|x| hex::encode(x.hash()))
                        .collect(),
                    sync_index: behaviour.blockchain.sync.index,
                    syncing: behaviour.blockchain.sync.syncing,
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_balance(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
    first: &str,
) -> Result<(), Box<dyn Error>> {
    let public_key = address::decode(
        BALANCE
            .find(first)
            .ok_or("GET BALANCE 1")?
            .as_str()
            .trim()
            .get(9..)
            .ok_or("GET BALANCE 2")?,
    )?;
    let balance = swarm
        .behaviour()
        .blockchain
        .states
        .dynamic
        .balance(&public_key);
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
                serde_json::to_string(&balance)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_balance_staked(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
    first: &str,
) -> Result<(), Box<dyn Error>> {
    let public_key = address::decode(
        BALANCE_STAKED
            .find(first)
            .ok_or("GET BALANCE_STAKED 1")?
            .as_str()
            .trim()
            .get(16..)
            .ok_or("GET BALANCE_STAKED 2")?,
    )?;
    let balance = swarm
        .behaviour()
        .blockchain
        .states
        .dynamic
        .balance_staked(&public_key);
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
                serde_json::to_string(&balance)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_height(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
) -> Result<(), Box<dyn Error>> {
    let height = swarm.behaviour().blockchain.height();
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
                serde_json::to_string(&height)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_hash_by_height(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
    first: &str,
) -> Result<(), Box<dyn Error>> {
    let height = HASH_BY_HEIGHT
        .find(first)
        .ok_or("GET HASH_BY_HEIGHT 1")?
        .as_str()
        .trim()
        .get(6..)
        .ok_or("GET HASH_BY_HEIGHT 2")?
        .parse::<types::Height>()?;
    let states = &swarm.behaviour().blockchain.states;
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
Content-Type: application/json

{}",
                serde_json::to_string(&hex::encode(hash))?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_block_by_hash(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
    first: &str,
) -> Result<(), Box<dyn Error>> {
    #[derive(Serialize)]
    struct Data {
        previous_hash: String,
        timestamp: types::Timestamp,
        public_key: String,
        signature: String,
        transactions: Vec<String>,
        stakes: Vec<String>,
    }
    let hash = hex::decode(
        BLOCK_BY_HASH
            .find(first)
            .ok_or("GET BLOCK_BY_HASH 1")?
            .as_str()
            .trim()
            .get(7..)
            .ok_or("GET BLOCK_BY_HASH 2")?,
    )?;
    let block = Block::get(&swarm.behaviour().blockchain.db, &hash)?;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
                serde_json::to_string(&Data {
                    previous_hash: hex::encode(&block.previous_hash),
                    timestamp: block.timestamp,
                    public_key: address::encode(&block.public_key),
                    signature: hex::encode(&block.signature),
                    transactions: block
                        .transactions
                        .iter()
                        .map(|x| hex::encode(&x.hash()))
                        .collect(),
                    stakes: block
                        .stakes
                        .iter()
                        .map(|x| hex::encode(&x.hash()))
                        .collect(),
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_transaction_by_hash(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
    first: &str,
) -> Result<(), Box<dyn Error>> {
    #[derive(Serialize)]
    struct Data {
        public_key_input: String,
        public_key_output: String,
        amount: types::Amount,
        fee: types::Amount,
        timestamp: types::Timestamp,
        signature: String,
    }
    let hash = hex::decode(
        TRANSACTION_BY_HASH
            .find(first)
            .ok_or("GET TRANSACTION_BY_HASH 1")?
            .as_str()
            .trim()
            .get(13..)
            .ok_or("GET TRANSACTION_BY_HASH 2")?,
    )?;
    let transaction = Transaction::get(&swarm.behaviour().blockchain.db, &hash)?;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
                serde_json::to_string(&Data {
                    public_key_input: address::encode(&transaction.public_key_input),
                    public_key_output: address::encode(&transaction.public_key_output),
                    amount: transaction.amount,
                    fee: transaction.fee,
                    timestamp: transaction.timestamp,
                    signature: hex::encode(&transaction.signature)
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_get_json_stake_by_hash(
    stream: &mut tokio::net::TcpStream,
    swarm: &Swarm<MyBehaviour>,
    first: &str,
) -> Result<(), Box<dyn Error>> {
    #[derive(Serialize)]
    struct Data {
        public_key: String,
        amount: types::Amount,
        deposit: bool,
        fee: types::Amount,
        timestamp: types::Timestamp,
        signature: String,
    }
    let hash = hex::decode(
        STAKE_BY_HASH
            .find(first)
            .ok_or("GET STAKE_BY_HASH 1")?
            .as_str()
            .trim()
            .get(7..)
            .ok_or("GET STAKE_BY_HASH 2")?,
    )?;
    let stake = Stake::get(&swarm.behaviour().blockchain.db, &hash)?;
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Content-Type: application/json

{}",
                serde_json::to_string(&Data {
                    public_key: address::encode(&stake.public_key),
                    amount: stake.amount,
                    deposit: stake.deposit,
                    fee: stake.fee,
                    timestamp: stake.timestamp,
                    signature: hex::encode(&stake.signature)
                })?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_post_json_transaction(
    stream: &mut tokio::net::TcpStream,
    swarm: &mut Swarm<MyBehaviour>,
    buffer: &[u8; 1024],
) -> Result<(), Box<dyn Error>> {
    let compressed: CompressedTransaction = bincode::deserialize(&hex::decode(
        buffer
            .lines()
            .nth(5)
            .ok_or("POST TRANSACTION 1")??
            .get(0..*TRANSACTION_SERIALIZED)
            .ok_or("POST TRANSACTION 2")?,
    )?)?;
    let behaviour = swarm.behaviour_mut();
    let status = match behaviour
        .blockchain
        .pending_transactions_push(Transaction::from(&compressed))
    {
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
Content-Type: application/json

{}",
                serde_json::to_string(&status)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn handler_post_json_stake(
    stream: &mut tokio::net::TcpStream,
    swarm: &mut Swarm<MyBehaviour>,
    buffer: &[u8; 1024],
) -> Result<(), Box<dyn Error>> {
    let compressed: CompressedStake = bincode::deserialize(&hex::decode(
        buffer
            .lines()
            .nth(5)
            .ok_or("POST STAKE 1")??
            .get(0..*STAKE_SERIALIZED)
            .ok_or("POST STAKE 2")?,
    )?)?;
    let behaviour = swarm.behaviour_mut();
    let status = match behaviour
        .blockchain
        .pending_stakes_push(Stake::from(&compressed))
    {
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
    stream
        .write_all("HTTP/1.1 404 NOT FOUND".as_bytes())
        .await?;
    Ok(())
}
