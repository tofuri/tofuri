use crate::{
    address,
    blockchain::Blockchain,
    cli::{ValidatorArgs, WalletArgs},
    transaction::Transaction,
    types,
};
use chrono::Local;
use colored::*;
use env_logger::Builder;
use log::{debug, error, info, Level, LevelFilter};
use std::{error::Error, io::Write};
use tokio::net::TcpListener;
pub fn clear() {
    print!("\x1B[2J\x1B[1;1H");
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
pub fn build() {
    info!("{} {}", "Version".cyan(), env!("CARGO_PKG_VERSION"));
    info!("{} {}", "Commit".cyan(), env!("GIT_HASH"));
    info!("{} {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY"));
}
pub fn blockchain(blockchain: &Blockchain) {
    info!(
        "{} {}",
        "PubKey".cyan(),
        address::encode(blockchain.get_keypair().public.as_bytes())
    );
    info!("{} {}", "Height".cyan(), blockchain.get_hashes().len());
    info!(
        "{} {}",
        "Pending txns".cyan(),
        blockchain.get_pending_transactions().len()
    );
    info!(
        "{} {}",
        "Pending stakes".cyan(),
        blockchain.get_pending_stakes().len()
    );
    info!("{} {}", "Validators".cyan(), blockchain.get_stakers().len());
}
pub fn validator_args(args: &ValidatorArgs) {
    info!("{} {}", "--debug".cyan(), args.debug);
    info!("{} {}", "--multiaddr".cyan(), args.multiaddr);
    info!("{} {}", "--tempdb".cyan(), args.tempdb);
    info!("{} {}", "--tempkey".cyan(), args.tempkey);
}
pub fn wallet_args(args: &WalletArgs) {
    info!("{} {}", "--api".cyan(), args.api);
}
pub fn http(listener: &TcpListener) -> Result<(), Box<dyn Error>> {
    info!(
        "{} http://{}",
        "Interface".cyan(),
        listener.local_addr()?.to_string().green()
    );
    Ok(())
}
pub fn pending_transactions(pending_transactions: &Vec<Transaction>) {
    info!(
        "{} {}",
        "Pending txns".magenta(),
        pending_transactions.len().to_string().yellow()
    );
}
pub fn err(err: Box<dyn Error>) {
    error!("{}", err.to_string().red());
}
pub fn http_handle(first: &str) {
    info!("{} {}", "Interface".cyan(), first.green());
}
pub fn p2p_event(event_type: &str, event: String) {
    info!("{} {}", event_type.cyan(), event)
}
pub fn heartbeat_lag(heartbeats: &types::Heartbeats, millis: f64) {
    debug!(
        "{} {} {}ms",
        "Heartbeat".cyan(),
        heartbeats,
        millis.to_string().yellow(),
    );
}
