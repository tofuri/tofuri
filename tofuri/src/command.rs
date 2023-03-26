use crate::Node;
use std::process;
use tofuri_address::address;
use tofuri_p2p::multiaddr;
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
    let args: Vec<&str> = line.trim().split(" ").collect();
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
    let address = address::encode(&node.key.address_bytes());
    info!(address)
}
fn peers(node: &mut Node) {
    info!("{:?}", node.p2p.connections.values().collect::<Vec<_>>());
}
fn balance(node: &mut Node, args: &[&str]) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return error!("{}", "Missing argument"),
    };
    let address_bytes = match address::decode(arg1) {
        Ok(x) => x,
        Err(_) => return error!("{}", "Invalid address"),
    };
    let balance = tofuri_int::to_string(node.blockchain.balance(&address_bytes));
    info!(balance);
}
fn staked(node: &mut Node, args: &[&str]) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return error!("{}", "Missing argument"),
    };
    let address_bytes = match address::decode(arg1) {
        Ok(x) => x,
        Err(_) => return error!("{}", "Invalid address"),
    };
    let staked = tofuri_int::to_string(node.blockchain.staked(&address_bytes));
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
    let multiaddr = multiaddr::from_ip_addr(&ip_addr);
    info!(multiaddr = multiaddr.to_string(), "Dialing");
    let _ = node.p2p.swarm.dial(multiaddr);
}
fn filter(args: &[&str], reload_handle: &reload::Handle<EnvFilter, Registry>) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return error!("{}", "Missing argument"),
    };
    let filter = EnvFilter::new(arg1);
    info!(filter = filter.to_string(), "Reload filter");
    reload_handle.modify(|x| *x = filter).unwrap();
}
