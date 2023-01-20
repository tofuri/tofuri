use crate::node::Node;
use crate::util;
use chrono::TimeZone;
use chrono::Utc;
use libp2p::Multiaddr;
use log::error;
use pea_address::address;
use pea_api as api;
use pea_core::*;
use pea_db as db;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::error::Error;
use std::io::BufRead;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;
fn parse_body(buffer: &[u8; 1024]) -> Result<String, Box<dyn Error>> {
    let str = std::str::from_utf8(buffer)?;
    let vec = str.split("\n\n").collect::<Vec<&str>>();
    let body = vec.get(1).ok_or("empty body")?;
    Ok(body.trim_end_matches(char::from(0)).to_string())
}
fn parse_request_line(buffer: &[u8]) -> Result<String, Box<dyn Error>> {
    Ok(buffer.lines().next().ok_or("empty request line")??)
}
pub async fn handler(mut stream: TcpStream, node: &mut Node) -> Result<(usize, String), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let bytes = timeout(Duration::from_millis(1), stream.read(&mut buffer)).await??;
    let request_line = parse_request_line(&buffer)?;
    let vec: Vec<&str> = request_line.split(' ').collect();
    let method = vec.first().ok_or("method")?;
    let path = vec.get(1).ok_or("path")?;
    let args: Vec<&str> = path.split('/').filter(|&x| !x.is_empty()).collect();
    write(
        &mut stream,
        match *method {
            "GET" => get(node, args),
            "POST" => post(node, args, parse_body(&buffer)?),
            _ => c405(),
        }?,
    )
    .await?;
    stream.flush().await?;
    Ok((bytes, request_line))
}
fn get(node: &mut Node, args: Vec<&str>) -> Result<String, Box<dyn Error>> {
    match args.first() {
        Some(a) => match *a {
            "info" => get_info(node),
            "sync" => get_sync(node),
            "dynamic" => get_dynamic(node),
            "trusted" => get_trusted(node),
            "options" => get_options(node),
            "balance" => match args.get(1) {
                Some(b) => match address::decode(b) {
                    Ok(c) => get_balance(node, c),
                    Err(_) => c400(),
                },
                None => c400(),
            },
            "staked" => match args.get(1) {
                Some(b) => match address::decode(b) {
                    Ok(c) => get_staked(node, c),
                    Err(_) => c400(),
                },
                None => c400(),
            },
            "height" => match args.get(1) {
                Some(b) => match hex::decode(b) {
                    Ok(c) => get_hash_height(node, c),
                    Err(_) => c400(),
                },
                None => get_height(node),
            },
            "hash" => match args.get(1) {
                Some(b) => match b.parse::<usize>() {
                    Ok(c) => get_height_hash(node, c),
                    Err(_) => c400(),
                },
                None => get_height(node),
            },
            "block" => match args.get(1) {
                Some(b) => match *b {
                    "latest" => get_block_latest(node),
                    c => match hex::decode(c) {
                        Ok(d) => get_block_by_hash(node, d),
                        Err(_) => c400(),
                    },
                },
                None => c400(),
            },
            "transaction" => match args.get(1) {
                Some(b) => match hex::decode(b) {
                    Ok(c) => get_transaction_by_hash(node, c),
                    Err(_) => c400(),
                },
                None => c400(),
            },
            "stake" => match args.get(1) {
                Some(b) => match hex::decode(b) {
                    Ok(c) => get_stake_by_hash(node, c),
                    Err(_) => c400(),
                },
                None => c400(),
            },
            "peer" => match args.get(1..) {
                Some(b) => get_peer(node, b),
                None => c400(),
            },
            "peers" => get_peers(node),
            _ => c404(),
        },
        None => get_index(),
    }
}
fn post(node: &mut Node, args: Vec<&str>, body: String) -> Result<String, Box<dyn Error>> {
    match args.first() {
        Some(a) => match *a {
            "transaction" => post_transaction(node, body),
            "stake" => post_stake(node, body),
            _ => c404(),
        },
        None => c400(),
    }
}
async fn write(stream: &mut TcpStream, string: String) -> Result<(), Box<dyn Error>> {
    stream.write_all(string.as_bytes()).await?;
    Ok(())
}
fn json(string: String) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{string}"
    )
}
fn text(string: String) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *

{string}"
    )
}
fn get_index() -> Result<String, Box<dyn Error>> {
    Ok(text(format!(
        "\
{} = {{ version = \"{}\" }}
{}/tree/{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
        env!("GIT_HASH"),
    )))
}
fn get_info(node: &mut Node) -> Result<String, Box<dyn Error>> {
    Ok(json(serde_json::to_string(&api::Info {
        time: Utc.timestamp_nanos(chrono::offset::Utc::now().timestamp_micros() * 1_000).to_rfc2822(),
        address: address::encode(&node.blockchain.key.address_bytes()),
        uptime: node.uptime(),
        heartbeats: node.heartbeats,
        tree_size: node.blockchain.tree.size(),
        lag: node.lag,
    })?))
}
fn get_sync(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let last = node.last_seen();
    let status = node.sync_status();
    Ok(json(serde_json::to_string(&api::Sync {
        status,
        height: node.blockchain.height(),
        last_seen: last,
    })?))
}
fn get_dynamic(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let dynamic = &node.blockchain.states.dynamic;
    Ok(json(serde_json::to_string(&api::Dynamic {
        random_queue: dynamic.stakers_n(8).iter().map(address::encode).collect(),
        hashes: dynamic.hashes.len(),
        latest_hashes: dynamic.hashes.iter().rev().take(16).map(hex::encode).collect(),
        stakers: dynamic.stakers.iter().take(16).map(address::encode).collect(),
    })?))
}
fn get_trusted(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let trusted = &node.blockchain.states.trusted;
    Ok(json(serde_json::to_string(&api::Trusted {
        hashes: trusted.hashes.len(),
        latest_hashes: trusted.hashes.iter().rev().take(16).map(hex::encode).collect(),
        stakers: trusted.stakers.iter().take(16).map(address::encode).collect(),
    })?))
}
fn get_options(node: &mut Node) -> Result<String, Box<dyn Error>> {
    Ok(json(serde_json::to_string(&api::Options {
        mint: node.mint,
        trust: node.blockchain.trust_fork_after_blocks,
        ban_offline: node.p2p_ban_offline,
        time_delta: node.blockchain.time_delta,
        max_established: node.max_established,
        tps: node.tps,
        bind_api: node.bind_api.clone(),
        host: node.p2p_host.clone(),
        tempdb: node.tempdb,
        tempkey: node.tempkey,
        dev: node.dev,
    })?))
}
fn get_balance(node: &mut Node, address_bytes: AddressBytes) -> Result<String, Box<dyn Error>> {
    let balance = node.blockchain.states.dynamic.balance(&address_bytes);
    Ok(json(serde_json::to_string(&pea_int::to_string(balance))?))
}
fn get_staked(node: &mut Node, address_bytes: AddressBytes) -> Result<String, Box<dyn Error>> {
    let balance = node.blockchain.states.dynamic.staked(&address_bytes);
    Ok(json(serde_json::to_string(&pea_int::to_string(balance))?))
}
fn get_height(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let height = node.blockchain.height();
    Ok(json(serde_json::to_string(&height)?))
}
fn get_hash_height(node: &mut Node, hash: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let block_c = db::block::get_c(&node.blockchain.db, &hash)?;
    let height = node.blockchain.tree.height(&block_c.previous_hash);
    Ok(json(serde_json::to_string(&height)?))
}
fn get_block_latest(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let block_a = &node.blockchain.states.dynamic.latest_block;
    Ok(json(serde_json::to_string(&api::Block {
        hash: hex::encode(block_a.hash),
        previous_hash: hex::encode(block_a.previous_hash),
        timestamp: block_a.timestamp,
        address: address::encode(&block_a.input_address()),
        signature: hex::encode(block_a.signature),
        pi: hex::encode(block_a.pi),
        beta: hex::encode(block_a.beta),
        transactions: block_a.transactions.iter().map(|x| hex::encode(x.hash)).collect(),
        stakes: block_a.stakes.iter().map(|x| hex::encode(x.hash)).collect(),
    })?))
}
fn get_height_hash(node: &mut Node, height: usize) -> Result<String, Box<dyn Error>> {
    let states = &node.blockchain.states;
    let hashes_trusted = &states.trusted.hashes;
    let hashes_dynamic = &states.dynamic.hashes;
    if height >= hashes_trusted.len() + hashes_dynamic.len() {
        return Err("GET HEIGHT_HASH".into());
    }
    let hash = if height < hashes_trusted.len() {
        hashes_trusted[height]
    } else {
        hashes_dynamic[height - hashes_trusted.len()]
    };
    Ok(json(serde_json::to_string(&hex::encode(hash))?))
}
fn get_block_by_hash(node: &mut Node, hash: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let block_a = db::block::get_a(&node.blockchain.db, &hash)?;
    Ok(json(serde_json::to_string(&api::Block {
        hash: hex::encode(block_a.hash),
        previous_hash: hex::encode(block_a.previous_hash),
        timestamp: block_a.timestamp,
        address: address::encode(&block_a.input_address()),
        signature: hex::encode(block_a.signature),
        pi: hex::encode(block_a.pi),
        beta: hex::encode(block_a.beta),
        transactions: block_a.transactions.iter().map(|x| hex::encode(x.hash)).collect(),
        stakes: block_a.stakes.iter().map(|x| hex::encode(x.hash)).collect(),
    })?))
}
fn get_transaction_by_hash(node: &mut Node, hash: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let transaction_a = db::transaction::get_a(&node.blockchain.db, &hash)?;
    Ok(json(serde_json::to_string(&api::Transaction {
        hash: hex::encode(transaction_a.hash),
        input_address: address::encode(&transaction_a.input_address),
        output_address: address::encode(&transaction_a.output_address),
        amount: pea_int::to_string(transaction_a.amount),
        fee: pea_int::to_string(transaction_a.fee),
        timestamp: transaction_a.timestamp,
        signature: hex::encode(transaction_a.signature),
    })?))
}
fn get_stake_by_hash(node: &mut Node, hash: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let stake_a = db::stake::get_a(&node.blockchain.db, &hash)?;
    Ok(json(serde_json::to_string(&api::Stake {
        hash: hex::encode(stake_a.hash),
        address: address::encode(&stake_a.input_address),
        fee: pea_int::to_string(stake_a.fee),
        deposit: stake_a.deposit,
        timestamp: stake_a.timestamp,
        signature: hex::encode(stake_a.signature),
    })?))
}
fn get_peers(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let peers: Vec<&Multiaddr> = node.p2p_connections.keys().collect();
    Ok(json(serde_json::to_string(&peers)?))
}
fn get_peer(node: &mut Node, slice: &[&str]) -> Result<String, Box<dyn Error>> {
    let multiaddr = format!("/{}", slice.join("/")).parse::<Multiaddr>()?;
    let multiaddr = pea_p2p::multiaddr::multiaddr_filter_ip_port(&multiaddr).ok_or("multiaddr filter_ip_port")?;
    let string = multiaddr.to_string();
    node.p2p_unknown.insert(multiaddr);
    Ok(text(string))
}
fn post_transaction(node: &mut Node, body: String) -> Result<String, Box<dyn Error>> {
    let transaction_b: TransactionB = serde_json::from_str(&body)?;
    let data = bincode::serialize(&transaction_b).unwrap();
    let status = match node.blockchain.pending_transactions_push(transaction_b, util::timestamp()) {
        Ok(()) => {
            if node.gossipsub_has_mesh_peers("transaction") {
                node.gossipsub_publish("transaction", data);
            }
            "success".to_string()
        }
        Err(err) => {
            error!("{}", err);
            err.to_string()
        }
    };
    Ok(json(serde_json::to_string(&status)?))
}
fn post_stake(node: &mut Node, body: String) -> Result<String, Box<dyn Error>> {
    let stake_b: StakeB = serde_json::from_str(&body)?;
    let data = bincode::serialize(&stake_b).unwrap();
    let status = match node.blockchain.pending_stakes_push(stake_b, util::timestamp()) {
        Ok(()) => {
            if node.gossipsub_has_mesh_peers("stake") {
                node.gossipsub_publish("stake", data);
            }
            "success".to_string()
        }
        Err(err) => {
            error!("{}", err);
            err.to_string()
        }
    };
    Ok(json(serde_json::to_string(&status)?))
}
fn c400() -> Result<String, Box<dyn Error>> {
    Ok("HTTP/1.1 400 Bad Request".to_string())
}
fn c404() -> Result<String, Box<dyn Error>> {
    Ok("HTTP/1.1 404 Not Found".to_string())
}
fn c405() -> Result<String, Box<dyn Error>> {
    Ok("HTTP/1.1 405 Method Not Allowed".to_string())
}
