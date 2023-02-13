use crate::Node;
use chrono::TimeZone;
use chrono::Utc;
use colored::*;
use libp2p::Multiaddr;
use log::error;
use log::info;
use pea_address::address;
use pea_api as api;
use pea_core::*;
use pea_db as db;
use pea_p2p::multiaddr;
use pea_stake::StakeB;
use pea_transaction::TransactionB;
use std::error::Error;
use std::io;
use std::io::BufRead;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;
pub async fn accept(node: &mut Node, res: Result<(TcpStream, SocketAddr), io::Error>) {
    match res {
        Ok((stream, socket_addr)) => match request(node, stream).await {
            Ok((bytes, first)) => info!(
                "{} {} {} {}",
                "API".cyan(),
                socket_addr.to_string().magenta(),
                bytes.to_string().yellow(),
                first
            ),
            Err(err) => error!("{} {} {}", "API".cyan(), socket_addr.to_string().magenta(), err),
        },
        Err(err) => error!("{} {}", "API".cyan(), err),
    }
}
async fn request(node: &mut Node, mut stream: TcpStream) -> Result<(usize, String), Box<dyn Error>> {
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
fn parse_body(buffer: &[u8; 1024]) -> Result<String, Box<dyn Error>> {
    let str = std::str::from_utf8(buffer)?;
    let vec = str.split("\n\n").collect::<Vec<&str>>();
    let body = vec.get(1).ok_or("empty body")?;
    Ok(body.trim_end_matches(char::from(0)).to_string())
}
fn parse_request_line(buffer: &[u8]) -> Result<String, Box<dyn Error>> {
    Ok(buffer.lines().next().ok_or("empty request line")??)
}
fn get(node: &mut Node, args: Vec<&str>) -> Result<String, Box<dyn Error>> {
    match args.first() {
        Some(a) => match *a {
            "info" => get_info(node),
            "sync" => get_sync(node),
            "dynamic" => get_dynamic(node),
            "trusted" => get_trusted(node),
            "args" => get_args(node),
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
                    Ok(c) => get_height_by_hash(node, c),
                    Err(_) => c400(),
                },
                None => get_height(node),
            },
            "hash" => match args.get(1) {
                Some(b) => match b.parse::<usize>() {
                    Ok(c) => get_hash_by_height(node, c),
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
        address: address::encode(&node.key.address_bytes()),
        uptime: pea_util::uptime(node.ticks as f64, node.args.tps),
        ticks: node.ticks,
        tree_size: node.blockchain.tree.size(),
        lag: node.lag,
    })?))
}
fn get_sync(node: &mut Node) -> Result<String, Box<dyn Error>> {
    Ok(json(serde_json::to_string(&api::Sync::from(&node.blockchain))?))
}
fn get_dynamic(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let dynamic = &node.blockchain.states.dynamic;
    Ok(json(serde_json::to_string(&api::Dynamic::from(&dynamic))?))
}
fn get_trusted(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let trusted = &node.blockchain.states.trusted;
    Ok(json(serde_json::to_string(&api::Trusted::from(&trusted))?))
}
fn get_args(node: &mut Node) -> Result<String, Box<dyn Error>> {
    Ok(json(serde_json::to_string(&node.args)?))
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
fn get_height_by_hash(node: &mut Node, hash: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let block_c = db::block::get_c(&node.db, &hash)?;
    let height = node.blockchain.tree.height(&block_c.previous_hash);
    Ok(json(serde_json::to_string(&height)?))
}
fn get_block_latest(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let block_a = &node.blockchain.states.dynamic.latest_block;
    Ok(json(serde_json::to_string(&api::Block::from(&block_a))?))
}
fn get_hash_by_height(node: &mut Node, height: usize) -> Result<String, Box<dyn Error>> {
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
    let block_a = db::block::get_a(&node.db, &hash)?;
    Ok(json(serde_json::to_string(&api::Block::from(&block_a))?))
}
fn get_transaction_by_hash(node: &mut Node, hash: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let transaction_a = db::transaction::get_a(&node.db, &hash)?;
    Ok(json(serde_json::to_string(&api::Transaction::from(&transaction_a))?))
}
fn get_stake_by_hash(node: &mut Node, hash: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let stake_a = db::stake::get_a(&node.db, &hash)?;
    Ok(json(serde_json::to_string(&api::Stake::from(&stake_a))?))
}
fn get_peers(node: &mut Node) -> Result<String, Box<dyn Error>> {
    let peers: Vec<&Multiaddr> = node.p2p.connections.keys().collect();
    Ok(json(serde_json::to_string(&peers)?))
}
fn get_peer(node: &mut Node, slice: &[&str]) -> Result<String, Box<dyn Error>> {
    let multiaddr = format!("/{}", slice.join("/")).parse::<Multiaddr>()?;
    let multiaddr = multiaddr::ip_port(&multiaddr).ok_or("multiaddr filter_ip_port")?;
    let string = multiaddr.to_string();
    node.p2p.unknown.insert(multiaddr);
    Ok(text(string))
}
fn post_transaction(node: &mut Node, body: String) -> Result<String, Box<dyn Error>> {
    let transaction_b: TransactionB = serde_json::from_str(&body)?;
    let data = bincode::serialize(&transaction_b).unwrap();
    let status = match node
        .blockchain
        .pending_transactions_push(&node.db, transaction_b, pea_util::timestamp(), node.args.time_delta)
    {
        Ok(()) => {
            if node.p2p.gossipsub_has_mesh_peers("transaction") {
                if let Err(err) = node.p2p.gossipsub_publish("transaction", data) {
                    error!("{}", err);
                }
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
    let status = match node
        .blockchain
        .pending_stakes_push(&node.db, stake_b, pea_util::timestamp(), node.args.time_delta)
    {
        Ok(()) => {
            if node.p2p.gossipsub_has_mesh_peers("stake") {
                if let Err(err) = node.p2p.gossipsub_publish("stake", data) {
                    error!("{}", err);
                }
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
