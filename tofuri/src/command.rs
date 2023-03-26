use crate::Node;
use std::process;
use tofuri_address::address;
use tofuri_p2p::multiaddr;
pub fn command(node: &mut Node, line: &mut String) {
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
        "dial" => dial(node, &args),
        _ => {}
    }
    line.clear();
}
fn stop() {
    println!("Stopping...");
    process::exit(0)
}
fn address(node: &mut Node) {
    println!("{}", address::encode(&node.key.address_bytes()))
}
fn peers(node: &mut Node) {
    println!("{:?}", node.p2p.connections.values().collect::<Vec<_>>());
}
fn balance(node: &mut Node, args: &[&str]) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return,
    };
    let address_bytes = address::decode(arg1).unwrap();
    let balance = node.blockchain.balance(&address_bytes);
    println!("{}", tofuri_int::to_string(balance));
}
fn dial(node: &mut Node, args: &[&str]) {
    let arg1 = match args.get(1) {
        Some(x) => *x,
        None => return,
    };
    let ip_addr = match arg1.parse() {
        Ok(x) => x,
        Err(_) => return,
    };
    let multiaddr = multiaddr::from_ip_addr(&ip_addr);
    println!("Dialing {}", multiaddr);
    let _ = node.p2p.swarm.dial(multiaddr);
}
