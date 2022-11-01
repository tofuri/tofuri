use crate::{
    constants::{DECIMAL_PRECISION, MIN_STAKE_MULTIPLIER},
    types,
};
use rand::rngs::OsRng;
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
pub fn keygen() -> types::Keypair {
    let mut csprng = OsRng {};
    types::Keypair::generate(&mut csprng)
}
pub fn timestamp() -> types::Timestamp {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as types::Timestamp
}
pub fn hash(input: &[u8]) -> types::Hash {
    blake3::hash(input).into()
}
pub fn read_lines(path: impl AsRef<Path>) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let buf = BufReader::new(file);
    Ok(buf.lines().map(|l| l.expect("Could not parse line")).collect())
}
pub fn reward(balance_staked: types::Amount) -> types::Amount {
    ((2f64.powf((balance_staked as f64 / DECIMAL_PRECISION as f64) / MIN_STAKE_MULTIPLIER as f64) - 1f64) * DECIMAL_PRECISION as f64) as types::Amount
}
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519::signature::{Signer, Verifier};
    use ed25519_dalek::{Digest, Keypair, PublicKey, SecretKey, Sha512, Signature};
    use test::Bencher;
    #[test]
    fn test_hash() {
        assert_eq!(blake3::hash(b"test").to_string(), "4878ca0425c739fa427f7eda20fe845f6b2e46ba5fe2a14df5b1e32f50603215".to_string());
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
    #[bench]
    fn bench_ed25519_dalek_sha512(b: &mut Bencher) {
        let message = &[0; 32];
        b.iter(|| {
            let mut prehashed: Sha512 = Sha512::new();
            prehashed.update(message);
            prehashed.finalize();
        });
    }
}
