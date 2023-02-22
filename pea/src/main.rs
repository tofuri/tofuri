use clap::Parser;
use colored::*;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
use log::debug;
use log::info;
use log::warn;
use pea::api_internal;
use pea::interval;
use pea::swarm;
use pea::Node;
use pea::CARGO_PKG_NAME;
use pea::CARGO_PKG_REPOSITORY;
use pea::CARGO_PKG_VERSION;
use pea_address::address;
use pea_blockchain::blockchain::Blockchain;
use pea_core::*;
use pea_key::Key;
use pea_p2p::multiaddr;
use pea_p2p::P2p;
use std::collections::HashSet;
use std::time::Duration;
use tempdir::TempDir;
use tokio::net::TcpListener;
use tokio::time::interval_at;
#[tokio::main]
async fn main() {
    let mut args = pea::Args::parse();
    pea_logger::init(args.debug);
    info!("{}", pea_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY));
    if args.dev {
        if args.tempdb == TEMP_DB {
            args.tempdb = DEV_TEMP_DB;
        }
        if args.tempkey == TEMP_KEY {
            args.tempkey = DEV_TEMP_KEY;
        }
        if args.api_internal == API_INTERNAL {
            args.api_internal = DEV_API_INTERNAL.to_string();
        }
        if args.host == HOST {
            args.host = DEV_HOST.to_string();
        }
    }
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--mint".cyan(), args.mint.to_string().magenta());
    info!("{} {}", "--trust".cyan(), args.trust.to_string().magenta());
    info!("{} {}", "--time-delta".cyan(), args.time_delta.to_string().magenta());
    info!("{} {}", "--max-established".cyan(), format!("{:?}", args.max_established).magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    info!("{} {}", "--peer".cyan(), args.peer.magenta());
    info!("{} {}", "--bind-api".cyan(), args.api_internal.magenta());
    info!("{} {}", "--host".cyan(), args.host.magenta());
    info!("{} {}", "--dev".cyan(), args.dev.to_string().magenta());
    if args.dev {
        warn!("{}", "DEVELOPMENT MODE IS ACTIVATED!".yellow());
    }
    let key = match args.tempkey {
        true => Key::generate(),
        false => pea_wallet::load(&args.wallet, &args.passphrase).unwrap().3,
    };
    info!("Address {}", address::encode(&key.address_bytes()).green());
    let tempdir = TempDir::new("peacash-db").unwrap();
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./peacash-db",
    };
    let db = pea_db::open(path);
    let mut known = HashSet::new();
    if let Some(multiaddr) = multiaddr::ip_port(&args.peer.parse::<Multiaddr>().unwrap()) {
        known.insert(multiaddr);
    }
    let peers = pea_db::peer::get_all(&db);
    for peer in peers {
        if let Some(multiaddr) = multiaddr::ip_port(&peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
    }
    let p2p = P2p::new(args.max_established, args.timeout, known).await.unwrap();
    let blockchain = Blockchain::default();
    let mut node = Node::new(db, key, args, p2p, blockchain);
    node.blockchain.load(&node.db, node.args.trust);
    info!(
        "Blockchain height is {}",
        if let Some(main) = node.blockchain.tree.main() {
            main.1.to_string().yellow()
        } else {
            "0".red()
        }
    );
    info!("Latest block seen {}", node.blockchain.last_seen().yellow());
    let multiaddr: Multiaddr = node.args.host.parse().unwrap();
    node.p2p.swarm.listen_on(multiaddr.clone()).unwrap();
    info!("Swarm is listening on {}", multiaddr.to_string().magenta());
    let listener = TcpListener::bind(&node.args.api_internal).await.unwrap();
    info!(
        "API is listening on {}{}",
        "http://".cyan(),
        listener.local_addr().unwrap().to_string().magenta()
    );
    let start = pea_util::interval_at_start();
    let mut interval_a = interval_at(start, Duration::from_secs(1));
    let mut interval_b = interval_at(start, Duration::from_millis(200));
    let mut interval_c = interval_at(start, Duration::from_secs(10));
    let mut interval_d = interval_at(start, Duration::from_secs(60));
    let mut interval_e = interval_at(start, Duration::from_secs(5));
    let mut interval_f = interval_at(start, Duration::from_secs(1));
    loop {
        let instant = tokio::select! {
            instant = interval_a.tick() => interval::grow(&mut node, instant),
            instant = interval_b.tick() => interval::sync_request(&mut node, instant),
            instant = interval_c.tick() => interval::share(&mut node, instant),
            instant = interval_d.tick() => interval::dial_known(&mut node, instant),
            instant = interval_e.tick() => interval::dial_unknown(&mut node, instant),
            instant = interval_f.tick() => interval::clear(&mut node, instant),
            event = node.p2p.swarm.select_next_some() => swarm::event(&mut node, event),
            res = listener.accept() => api_internal::accept(&mut node, res).await
        };
        let elapsed = instant.elapsed();
        node.lag = elapsed.as_micros() as f64 / 1_000_f64;
        node.ticks += 1;
        debug!("{} {} {}", "Tick".cyan(), node.ticks, format!("{elapsed:?}").yellow());
    }
}
