use pea_key::Key;
use pea_vrf::validate_key;
use pea_vrf::{prove, verify};
use sha2::Sha256;
use sha2::Sha512;
fn main() {
    let key = Key::generate();
    let mut alpha = [0; 32];
    for _ in 0..3 {
        for _ in 0..2 {
            let (beta, _) = prove::<Sha512, Sha256>(&alpha, &key.scalar);
            println!("{}", hex::encode(beta));
        }
        let (beta, pi) = prove::<Sha512, Sha256>(&alpha, &key.scalar);
        let public = key.compressed_ristretto().to_bytes();
        println!("{}", hex::encode(beta));
        println!("{}", validate_key(&public) && verify::<Sha512, Sha256>(&public, &alpha, &beta, &pi));
        alpha = beta;
    }
}
