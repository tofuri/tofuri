use clap::Parser;
use ed25519_dalek::Keypair;
use merkle_cbt::{merkle_tree::Merge, CBMT as ExCBMT};
use rand::rngs::OsRng;
use std::time::{SystemTime, UNIX_EPOCH};
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
pub mod print {
    use super::*;
    use crate::{
        blockchain::Blockchain, transaction::Transaction, validator::Validator, wallet::address,
    };
    use colored::*;
    use std::error::Error;
    use tokio::net::TcpListener;
    pub fn build() {
        println!("{}", "=== Build ===".magenta());
        println!("{}: {}", "Version".cyan(), env!("CARGO_PKG_VERSION"));
        println!("{}: {}", "Commit".cyan(), env!("GIT_HASH"));
        println!("{}: {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY"));
    }
    pub fn validator(validator: &Validator) {
        println!("{}", "=== Validator ===".magenta());
        println!(
            "{}: {}",
            "PubKey".cyan(),
            address::encode(&validator.keypair.public.as_bytes())
        );
        println!("{}: {}", "Peers".cyan(), validator.multiaddrs.len());
    }
    pub fn blockchain(blockchain: &Blockchain) {
        println!("{}", "=== Blockchain ===".magenta());
        println!("{}: {}", "Height".cyan(), blockchain.latest_height());
        println!(
            "{}: {}",
            "Pending txns".cyan(),
            blockchain.pending_transactions.len()
        );
        println!(
            "{}: {}",
            "Pending stakes".cyan(),
            blockchain.pending_stakes.len()
        );
        println!(
            "{}: {}",
            "Validators".cyan(),
            blockchain.stakers.queue.len()
        );
    }
    pub fn validator_args(args: &ValidatorArgs) {
        println!("{}", "=== Args ===".magenta());
        println!("{}: {}", "--log-level".cyan(), args.log_level);
        println!("{}: {}", "--multiaddr".cyan(), args.multiaddr);
        println!("{}: {}", "--tempdb".cyan(), args.tempdb);
    }
    pub fn wallet_args(args: &WalletArgs) {
        println!("{}", "=== Args ===".magenta());
        println!("{}: {}", "--api".cyan(), args.api);
    }
    pub fn http(listener: &TcpListener) -> Result<(), Box<dyn Error>> {
        println!("{}", "=== Interface ===".magenta());
        println!("http://{}", listener.local_addr()?.to_string().green());
        Ok(())
    }
    pub fn listen() {
        println!("{}", "=== Listening ===".magenta());
    }
    pub fn pending_transactions(pending_transactions: &Vec<Transaction>) {
        println!(
            "{}: {}",
            "Pending txns".magenta(),
            pending_transactions.len().to_string().yellow()
        );
    }
    pub fn err(err: Box<dyn Error>) {
        println!("{}", err.to_string().red());
    }
    pub fn http_api_request_handler(first: &str) {
        println!("{}: {}", "Interface".cyan(), first.green());
    }
    pub fn p2p_event(event_type: &str, event: String) {
        println!("{} {} {}", event_type.cyan(), "->".magenta(), event)
    }
    pub fn heartbeat_lag(heartbeats: usize) {
        let mut micros = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let secs = micros / 1_000_000;
        micros -= secs * 1_000_000;
        let millis = micros as f64 / 1_000 as f64;
        println!(
            "{}: {} {}ms",
            "Heartbeat".cyan(),
            heartbeats,
            millis.to_string().yellow(),
        );
    }
}
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct ValidatorArgs {
    /// Filter amount of logs
    #[clap(short, long, value_parser, default_value_t = 1)]
    pub log_level: u8,
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
