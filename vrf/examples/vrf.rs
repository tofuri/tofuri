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
            let proof = prove::<Sha512, Sha256>(&key.scalar, &alpha);
            let beta = proof.hash::<Sha256>();
            println!("{}", hex::encode(beta));
        }
        let proof = prove::<Sha512, Sha256>(&key.scalar, &alpha);
        let beta = proof.hash::<Sha256>();
        let pi = proof.to_bytes();
        let public = key.compressed_ristretto().to_bytes();
        println!("{}", hex::encode(beta));
        println!("{}", validate_key(&public) && verify::<Sha512, Sha256>(&public, &alpha, &pi, &beta));
        alpha = beta;
    }
}
