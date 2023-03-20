use clap::Parser;
use colored::*;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
use std::collections::HashSet;
use std::time::Duration;
use tempdir::TempDir;
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
use tofuri_p2p::multiaddr;
use tofuri_p2p::P2p;
use tokio::net::TcpListener;
use tokio::time::MissedTickBehavior::Skip;
use tracing::info;
use tracing::metadata::LevelFilter;
use tracing::warn;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy())
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let mut args = tofuri::Args::parse();
    info!("{}", tofuri_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY));
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
        false => tofuri_wallet::load(&args.wallet, &args.passphrase).unwrap().3,
    };
    info!(address = address::encode(&key.address_bytes()));
    let tempdir = TempDir::new("tofuri-db").unwrap();
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./tofuri-db",
    };
    let db = tofuri_db::open(path);
    let mut known = HashSet::new();
    if let Some(multiaddr) = multiaddr::ip_port(&args.peer.parse::<Multiaddr>().unwrap()) {
        known.insert(multiaddr);
    }
    let peers = tofuri_db::peer::get_all(&db);
    for peer in peers {
        if let Some(multiaddr) = multiaddr::ip_port(&peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
    }
    let p2p = P2p::new(args.max_established, args.timeout, known).await.unwrap();
    let blockchain = Blockchain::default();
    let mut node = Node::new(db, key, args.clone(), p2p, blockchain);
    node.blockchain.load(&node.db, node.args.trust);
    let multiaddr: Multiaddr = node.args.host.parse().unwrap();
    info!(multiaddr = multiaddr.to_string(), "P2P");
    node.p2p.swarm.listen_on(multiaddr).unwrap();
    let listener = TcpListener::bind(&node.args.rpc).await.unwrap();
    info!(local_addr = listener.local_addr().unwrap().to_string(), "RPC");
    let mut interval_a = tofuri_util::interval_at(Duration::from_secs(BLOCK_TIME as u64));
    let mut interval_b = tofuri_util::interval_at(Duration::from_millis(args.interval_b));
    let mut interval_c = tofuri_util::interval_at(Duration::from_millis(args.interval_c));
    let mut interval_d = tofuri_util::interval_at(Duration::from_millis(args.interval_d));
    let mut interval_e = tofuri_util::interval_at(Duration::from_millis(args.interval_e));
    let mut interval_f = tofuri_util::interval_at(Duration::from_millis(args.interval_f));
    interval_a.set_missed_tick_behavior(Skip);
    interval_b.set_missed_tick_behavior(Skip);
    interval_c.set_missed_tick_behavior(Skip);
    interval_d.set_missed_tick_behavior(Skip);
    interval_e.set_missed_tick_behavior(Skip);
    interval_f.set_missed_tick_behavior(Skip);
    loop {
        node.ticks += 1;
        tokio::select! {
            _ = interval_a.tick() => interval::grow(&mut node),
            _ = interval_b.tick() => interval::sync_request(&mut node),
            _ = interval_c.tick() => interval::share(&mut node),
            _ = interval_d.tick() => interval::dial_known(&mut node),
            _ = interval_e.tick() => interval::dial_unknown(&mut node),
            _ = interval_f.tick() => interval::clear(&mut node),
            event = node.p2p.swarm.select_next_some() => swarm::event(&mut node, event),
            res = listener.accept() => rpc::accept(&mut node, res).await
        }
    }
}
