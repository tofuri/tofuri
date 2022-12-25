use pea_key::Key;
use pea_vrf::validate_key;
use pea_vrf::{prove, verify};
use sha3::Sha3_224;
use sha3::Sha3_256;
use sha3::Sha3_512;
fn main() {
    let key = Key::generate();
    let secret = key.scalar;
    let public = key.compressed_ristretto().to_bytes();
    let alpha = [];
    let proof = prove::<Sha3_512, Sha3_256>(&secret, &alpha);
    let beta = proof.hash::<Sha3_224>();
    let pi = proof.to_bytes();
    println!("public {}", hex::encode(public));
    println!("beta {}", hex::encode(beta));
    println!("pi {}", hex::encode(pi));
    println!(
        "verify {}",
        validate_key(&public) && verify::<Sha3_512, Sha3_256, Sha3_224>(&public, &alpha, &pi, &beta)
    );
}
