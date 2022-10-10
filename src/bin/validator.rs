use chrono::Local;
use clap::Parser;
use colored::*;
use env_logger::Builder;
use log::{info, Level, LevelFilter};
use pea::{cli::ValidatorArgs, wallet::Wallet};
use pea_address as address;
use pea_node::{blockchain::Blockchain, db, p2p};
use std::{error::Error, io::Write};
use tempdir::TempDir;
use tokio::net::TcpListener;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = ValidatorArgs::parse();
    env_logger_init(args.debug);
    print_build();
    print_validator_args(&args);
    let tempdir = TempDir::new("rocksdb")?;
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./db",
    };
    let db = db::open(path);
    let wallet = match args.tempkey {
        true => Wallet::new(),
        false => Wallet::import(&args.wallet, &args.passphrase)?,
    };
    let mut blockchain = Blockchain::new(db, wallet.keypair);
    blockchain.load();
    print_blockchain(&blockchain);
    let mut swarm = p2p::swarm(blockchain).await?;
    swarm.listen_on(args.multiaddr.parse()?)?;
    let listener = TcpListener::bind(args.http).await?;
    print_http(&listener)?;
    p2p::listen(&mut swarm, listener).await?;
    Ok(())
}
pub fn colored_level(level: Level) -> ColoredString {
    match level {
        Level::Error => level.to_string().red(),
        Level::Warn => level.to_string().yellow(),
        Level::Info => level.to_string().green(),
        Level::Debug => level.to_string().blue(),
        Level::Trace => level.to_string().magenta(),
    }
}
pub fn env_logger_init(log_path: bool) {
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
        builder.format(|buf, record| {
            writeln!(
                buf,
                "[{} {}] {}",
                Local::now().format("%H:%M:%S"),
                colored_level(record.level()),
                record.args()
            )
        });
    }
    builder.filter(None, LevelFilter::Info).init();
}
pub fn print_build() {
    info!("{} {}", "Version".cyan(), env!("CARGO_PKG_VERSION"));
    info!("{} {}", "Commit".cyan(), env!("GIT_HASH"));
    info!("{} {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY"));
}
pub fn print_blockchain(blockchain: &Blockchain) {
    info!(
        "{} {}",
        "PubKey".cyan(),
        address::public::encode(blockchain.keypair.public.as_bytes())
    );
    let mut height = 0;
    if let Some(main) = blockchain.tree.main() {
        height = main.1;
    }
    info!("{} {}", "Height".cyan(), height);
    info!(
        "{} {}",
        "Pending txns".cyan(),
        blockchain.pending_transactions.len()
    );
    info!(
        "{} {}",
        "Pending stakes".cyan(),
        blockchain.pending_stakes.len()
    );
    info!(
        "{} {}",
        "Stakers".cyan(),
        blockchain.states.dynamic.stakers.len()
    );
}
pub fn print_validator_args(args: &ValidatorArgs) {
    info!("{} {}", "--debug".cyan(), args.debug);
    info!("{} {}", "--multiaddr".cyan(), args.multiaddr);
    info!("{} {}", "--tempdb".cyan(), args.tempdb);
    info!("{} {}", "--tempkey".cyan(), args.tempkey);
}
pub fn print_http(listener: &TcpListener) -> Result<(), Box<dyn Error>> {
    info!(
        "{} http://{}",
        "Interface".cyan(),
        listener.local_addr()?.to_string().green()
    );
    Ok(())
}
