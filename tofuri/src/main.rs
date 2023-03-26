use clap::Parser;
use colored::*;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
use std::collections::HashSet;
use std::time::Duration;
use tempdir::TempDir;
use tofuri::command;
use tofuri::interval;
use tofuri::rpc;
use tofuri::swarm;
use tofuri::Node;
use tofuri::CARGO_PKG_NAME;
use tofuri::CARGO_PKG_REPOSITORY;
use tofuri::CARGO_PKG_VERSION;
use tofuri_address::address;
use tofuri_blockchain::Blockchain;
use tofuri_core::*;
use tofuri_key::Key;
use tofuri_p2p::P2p;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tracing::info;
use tracing::warn;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
#[tokio::main]
async fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let (layer, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(layer)
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let mut args = tofuri::Args::parse();
    info!(
        "{}",
        tofuri_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY)
    );
    if args.dev {
        if args.tempdb == TEMP_DB {
            args.tempdb = TEMP_DB_DEV;
        }
        if args.tempkey == TEMP_KEY {
            args.tempkey = TEMP_KEY_DEV;
        }
        if args.rpc == RPC {
            args.rpc = RPC_DEV.to_string();
        }
        if args.host == HOST {
            args.host = HOST_DEV.to_string();
        }
    }
    info!("{:#?}", args);
    if args.dev {
        warn!("{}", "DEVELOPMENT MODE IS ACTIVATED!".yellow());
    }
    let key = match args.tempkey {
        true => Key::generate(),
        false => {
            tofuri_wallet::load(&args.wallet, &args.passphrase)
                .unwrap()
                .3
        }
    };
    info!(address = address::encode(&key.address_bytes()));
    let tempdir = TempDir::new("tofuri-db").unwrap();
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./tofuri-db",
    };
    let db = tofuri_db::open(path);
    let mut connections_known = HashSet::new();
    if let Ok(ip_addr) = args.peer.parse() {
        connections_known.insert(ip_addr);
    }
    let peers = tofuri_db::peer::get_all(&db).unwrap();
    for ip_addr in peers {
        connections_known.insert(ip_addr);
    }
    let p2p = P2p::new(args.max_established, args.timeout, connections_known)
        .await
        .unwrap();
    let blockchain = Blockchain::default();
    let mut node = Node::new(db, key, args.clone(), p2p, blockchain);
    node.blockchain.load(&node.db, node.args.trust).unwrap();
    let multiaddr: Multiaddr = node.args.host.parse().unwrap();
    info!(%multiaddr, "P2P");
    node.p2p.swarm.listen_on(multiaddr).unwrap();
    let listener = TcpListener::bind(&node.args.rpc).await.unwrap();
    info!(
        local_addr = listener.local_addr().unwrap().to_string(),
        "RPC"
    );
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut line = String::new();
    let mut interval_1s = tofuri_util::interval_at(Duration::from_secs(1));
    let mut interval_10s = tofuri_util::interval_at(Duration::from_secs(10));
    let mut interval_1m = tofuri_util::interval_at(Duration::from_secs(60));
    let mut interval_10m = tofuri_util::interval_at(Duration::from_secs(600));
    loop {
        node.ticks += 1;
        tokio::select! {
            _ = interval_1s.tick() => interval::interval_1s(&mut node),
            _ = interval_10s.tick() => interval::interval_10s(&mut node),
            _ = interval_1m.tick() => interval::interval_1m(&mut node),
            _ = interval_10m.tick() => interval::interval_10m(&mut node),
            event = node.p2p.swarm.select_next_some() => swarm::event(&mut node, event),
            res = listener.accept() => rpc::accept(&mut node, res).await,
            _ = reader.read_line(&mut line) => command::command(&mut node, &mut line, &reload_handle),
        }
    }
}
