use crate::p2p::multiaddr;
use crate::Node;
use decimal::Decimal;
use std::process;
use tofuri_address::public;
use tracing::error;
use tracing::info;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
pub fn command(
    node: &mut Node,
    line: &mut String,
    reload_handle: &reload::Handle<EnvFilter, Registry>,
) {
    let args: Vec<&str> = line.trim().split(' ').collect();
    let command = match args.first() {
        Some(x) => *x,
        None => return,
    };
    match command {
        "stop" => stop(),
        "address" => address(node),
        "peers" => peers(node),
        "balance" => balance(node, &args),
        "staked" => staked(node, &args),
        "dial" => dial(node, &args),
        "filter" => filter(&args, reload_handle),
        _ => {}
    }
    line.clear();
}
fn stop() {
    info!("Stopping...");
    process::exit(0)
}
fn address(node: &mut Node) {
    let address = match &node.key {
        Some(key) => public::encode(&key.address_bytes()),
        None => return error!("{}", "No key"),
    };
    info!(address)
}
fn peers(node: &mut Node) {
    let vec = node.p2p.connections.values().collect::<Vec<_>>();
    info!(?vec);
}
fn balance(node: &mut Node, args: &[&str]) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return error!("{}", "Missing argument"),
    };
    let address_bytes = match public::decode(arg1) {
        Ok(x) => x,
        Err(_) => return error!("{}", "Invalid address"),
    };
    let balance = node.blockchain.balance(&address_bytes).decimal::<18>();
    info!(balance);
}
fn staked(node: &mut Node, args: &[&str]) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return error!("{}", "Missing argument"),
    };
    let address_bytes = match public::decode(arg1) {
        Ok(x) => x,
        Err(_) => return error!("{}", "Invalid address"),
    };
    let staked = node.blockchain.staked(&address_bytes).decimal::<18>();
    info!(staked);
}
fn dial(node: &mut Node, args: &[&str]) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return error!("{}", "Missing argument"),
    };
    let ip_addr = match arg1.parse() {
        Ok(x) => x,
        Err(_) => return error!("{}", "Invalid IP address"),
    };
    let multiaddr = multiaddr::from_ip_addr(&ip_addr, node.args.testnet);
    info!(?multiaddr, "Dialing");
    let _ = node.p2p.swarm.dial(multiaddr);
}
fn filter(args: &[&str], reload_handle: &reload::Handle<EnvFilter, Registry>) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return error!("{}", "Missing argument"),
    };
    let filter = EnvFilter::new(arg1);
    info!(?filter, "Reload");
    if let Err(e) = reload_handle.modify(|x| *x = filter) {
        error!(?e);
    }
}
