use pea_key::Key;
use pea_vrf::validate_key;
use pea_vrf::{prove, verify};
use sha2::Sha224;
use sha2::Sha256;
use sha2::Sha512;
fn main() {
    let key = Key::generate();
    let secret = key.scalar;
    let public = key.compressed_ristretto().to_bytes();
    let alpha = [0; 32];
    let proof = prove::<Sha512, Sha256>(&secret, &alpha);
    let beta = proof.hash::<Sha224>();
    let pi = proof.to_bytes();
    println!("public {}", hex::encode(public));
    println!("beta {}", hex::encode(beta));
    println!("pi {}", hex::encode(pi));
    println!(
        "verify {}",
        validate_key(&public) && verify::<Sha512, Sha256, Sha224>(&public, &alpha, &pi, &beta)
    );
}
