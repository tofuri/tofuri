use chrono::Local;
use clap::Parser;
use colored::*;
use env_logger::Builder;
use libp2p::Multiaddr;
use log::{info, Level, LevelFilter};
use pea_address as address;
use pea_db as db;
use pea_node::{blockchain::Blockchain, p2p};
use pea_wallet::Wallet;
use std::{error::Error, io::Write};
use tempdir::TempDir;
use tokio::net::TcpListener;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// Multiaddr to listen on
    #[clap(short, long, value_parser, default_value = "/ip4/0.0.0.0/tcp/9333")]
    pub host: String,
    /// Multiaddr to dial
    #[clap(short, long, value_parser, default_value = "")]
    pub peer: String,
    /// TCP socket address to bind to
    #[clap(long, value_parser, default_value = "")]
    pub http_api: String,
    /// Store blockchain in a temporary database
    #[clap(long, value_parser, default_value_t = false)]
    pub tempdb: bool,
    /// Use temporary random keypair
    #[clap(long, value_parser, default_value_t = false)]
    pub tempkey: bool,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    env_logger_init(args.debug);
    info!("{} {}", "Version".cyan(), env!("CARGO_PKG_VERSION").yellow());
    info!("{} {}", "Commit".cyan(), env!("GIT_HASH").yellow());
    info!("{} {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY").yellow());
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--host".cyan(), args.host.magenta());
    info!("{} {}", "--peer".cyan(), args.peer.magenta());
    info!("{} {}", "--http-api".cyan(), args.http_api.magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    let tempdir = TempDir::new("peacash")?;
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./peacash/db",
    };
    let db = db::open(path);
    let wallet = match args.tempkey {
        true => Wallet::new(),
        false => Wallet::import(&args.wallet, &args.passphrase)?,
    };
    info!("{} {}", "PubKey".cyan(), address::public::encode(wallet.keypair.public.as_bytes()).green());
    let mut blockchain = Blockchain::new(db, wallet.keypair);
    let peers = db::peer::get_all(&blockchain.db);
    info!("{} {}", "Peers".cyan(), format!("{:?}", peers).yellow());
    blockchain.load();
    let mut swarm = p2p::swarm(blockchain).await?;
    swarm.listen_on(args.host.parse()?)?;
    swarm.dial(args.peer.parse::<Multiaddr>()?)?;
    for peer in peers {
        swarm.dial(peer.parse::<Multiaddr>()?)?;
    }
    let tcp_listener_http_api = if args.http_api != "" { Some(TcpListener::bind(args.http_api).await?) } else { None };
    p2p::listen(&mut swarm, tcp_listener_http_api).await?;
    Ok(())
}
fn env_logger_init(log_path: bool) {
    fn colored_level(level: Level) -> ColoredString {
        match level {
            Level::Error => level.to_string().red(),
            Level::Warn => level.to_string().yellow(),
            Level::Info => level.to_string().green(),
            Level::Debug => level.to_string().blue(),
            Level::Trace => level.to_string().magenta(),
        }
    }
    let mut builder = Builder::new();
    if log_path {
        builder.format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {}{}{}] {}",
                Local::now().format("%H:%M:%S"),
                colored_level(record.level()),
                record.file_static().unwrap().black(),
                ":".black(),
                record.line().unwrap().to_string().black(),
                record.args()
            )
        });
    } else {
        builder.format(|buf, record| writeln!(buf, "[{} {}] {}", Local::now().format("%H:%M:%S"), colored_level(record.level()), record.args()));
    }
    builder.filter(None, LevelFilter::Info).init();
}
