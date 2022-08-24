use clap::Parser;
use ed25519_dalek::Keypair;
use merkle_cbt::{merkle_tree::Merge, CBMT as ExCBMT};
use rand::rngs::OsRng;
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
pub fn keygen() -> Keypair {
    let mut csprng = OsRng {};
    Keypair::generate(&mut csprng)
}
pub struct Hasher;
impl Merge for Hasher {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut hasher = blake3::Hasher::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
}
pub type CBMT = ExCBMT<[u8; 32], Hasher>;
pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}
pub fn hash(input: &[u8]) -> [u8; 32] {
    blake3::hash(input).into()
}
pub fn read_lines(path: impl AsRef<Path>) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let buf = BufReader::new(file);
    Ok(buf
        .lines()
        .map(|l| l.expect("Could not parse line"))
        .collect())
}
pub mod print {
    use super::*;
    use crate::{
        blockchain::Blockchain, transaction::Transaction, validator::Validator, wallet::address,
    };
    use chrono::Local;
    use colored::*;
    use env_logger::Builder;
    use libp2p::Multiaddr;
    use log::{debug, error, info, warn, Level, LevelFilter};
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
                    "[{} {} {}{}{}]: {}",
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
                    "[{} {}]: {}",
                    Local::now().format("%H:%M:%S"),
                    colored_level(record.level()),
                    record.args()
                )
            });
        }
        builder.filter(None, LevelFilter::Info).init();
    }
    pub fn build() {
        info!("{}: {}", "Version".cyan(), env!("CARGO_PKG_VERSION"));
        info!("{}: {}", "Commit".cyan(), env!("GIT_HASH"));
        info!("{}: {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY"));
    }
    pub fn validator(validator: &Validator) {
        info!(
            "{}: {}",
            "PubKey".cyan(),
            address::encode(validator.keypair.public.as_bytes())
        );
        info!("{}: {}", "Peers".cyan(), validator.multiaddrs.len());
    }
    pub fn blockchain(blockchain: &Blockchain) {
        info!("{}: {}", "Height".cyan(), blockchain.latest_height());
        info!(
            "{}: {}",
            "Pending txns".cyan(),
            blockchain.pending_transactions.len()
        );
        info!(
            "{}: {}",
            "Pending stakes".cyan(),
            blockchain.pending_stakes.len()
        );
        info!(
            "{}: {}",
            "Validators".cyan(),
            blockchain.stakers.queue.len()
        );
    }
    pub fn validator_args(args: &ValidatorArgs) {
        info!("{}: {}", "--debug".cyan(), args.debug);
        info!("{}: {}", "--multiaddr".cyan(), args.multiaddr);
        info!("{}: {}", "--tempdb".cyan(), args.tempdb);
        info!("{}: {}", "--tempkey".cyan(), args.tempkey);
    }
    pub fn wallet_args(args: &WalletArgs) {
        info!("{}: {}", "--api".cyan(), args.api);
    }
    pub fn http(listener: &TcpListener) -> Result<(), Box<dyn Error>> {
        info!(
            "{}: http://{}",
            "Interface".cyan(),
            listener.local_addr()?.to_string().green()
        );
        Ok(())
    }
    pub fn pending_transactions(pending_transactions: &Vec<Transaction>) {
        info!(
            "{}: {}",
            "Pending txns".magenta(),
            pending_transactions.len().to_string().yellow()
        );
    }
    pub fn err(err: Box<dyn Error>) {
        error!("{}", err.to_string().red());
    }
    pub fn http_api_request_handler(first: &str) {
        info!("{}: {}", "Interface".cyan(), first.green());
    }
    pub fn p2p_event(event_type: &str, event: String) {
        info!("{}: {}", event_type.cyan(), event)
    }
    pub fn heartbeat_lag(heartbeats: usize, millis: f64) {
        debug!(
            "{}: {} {}ms",
            "Heartbeat".cyan(),
            heartbeats,
            millis.to_string().yellow(),
        );
    }
    pub fn known_peers(known: &Vec<Multiaddr>) {
        if known.is_empty() {
            warn!("{}", "Known peers list is empty!".yellow());
            return;
        }
        for multiaddr in known.iter() {
            info!("{}: {}", "Known peer".cyan(), multiaddr);
        }
    }
}
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct ValidatorArgs {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// Multiaddr to a validator in the network
    #[clap(short, long, value_parser, default_value = "/ip4/0.0.0.0/tcp/0")]
    pub multiaddr: String,
    /// Store blockchain in a temporary database
    #[clap(long, value_parser, default_value_t = false)]
    pub tempdb: bool,
    /// Multiaddr to a validator in the network
    #[clap(long, value_parser, default_value = ":::8080")]
    pub http: String,
    /// Use temporary random keypair
    #[clap(long, value_parser, default_value_t = false)]
    pub tempkey: bool,
    /// Path to list of known peers
    #[clap(long, value_parser, default_value = "./known.txt")]
    pub known: String,
}
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct WalletArgs {
    /// Multiaddr to a validator in the network
    #[clap(long, value_parser, default_value = "http://localhost:8080")]
    pub api: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519::signature::{Signer, Verifier};
    use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature};
    use test::Bencher;
    #[test]
    fn test_hash() {
        assert_eq!(
            blake3::hash(b"test").to_string(),
            "4878ca0425c739fa427f7eda20fe845f6b2e46ba5fe2a14df5b1e32f50603215".to_string()
        );
    }
    #[bench]
    fn bench_hash(b: &mut Bencher) {
        b.iter(|| hash(b"test"));
    }
    #[bench]
    fn bench_ed25519_dalek_sign(b: &mut Bencher) {
        let keypair = keygen();
        let message: &[u8] = &[0; 32];
        b.iter(|| keypair.sign(message));
    }
    #[bench]
    fn bench_ed25519_dalek_verify(b: &mut Bencher) {
        let keypair = keygen();
        let message: &[u8] = &[0, 32];
        let signature: Signature = keypair.try_sign(message).unwrap();
        b.iter(|| keypair.public.verify(message, &signature));
    }
    #[bench]
    fn bench_ed25519_dalek_verify_strict(b: &mut Bencher) {
        let keypair = keygen();
        let message: &[u8] = &[0, 32];
        let signature: Signature = keypair.try_sign(message).unwrap();
        b.iter(|| keypair.public.verify_strict(message, &signature));
    }
    #[bench]
    fn bench_ed25519_dalek_keypair(b: &mut Bencher) {
        let keypair = keygen();
        let keypair_bytes = keypair.to_bytes();
        b.iter(|| Keypair::from_bytes(&keypair_bytes));
    }
    #[bench]
    fn bench_ed25519_dalek_secret_key(b: &mut Bencher) {
        let keypair = keygen();
        let secret_key_bytes = keypair.secret.to_bytes();
        b.iter(|| SecretKey::from_bytes(&secret_key_bytes));
    }
    #[bench]
    fn bench_ed25519_dalek_public_key(b: &mut Bencher) {
        let keypair = keygen();
        let public_key_bytes = keypair.public.to_bytes();
        b.iter(|| PublicKey::from_bytes(&public_key_bytes));
    }
    #[bench]
    fn bench_ed25519_dalek_signature(b: &mut Bencher) {
        let keypair = keygen();
        let message: &[u8] = &[0, 32];
        let signature: Signature = keypair.try_sign(message).unwrap();
        let signature_bytes = signature.to_bytes();
        b.iter(|| Signature::try_from(signature_bytes));
    }
}
