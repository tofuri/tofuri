use pea_key::Key;
use pea_vrf::Proof;
use sha3::Sha3_224;
use sha3::Sha3_256;
use sha3::Sha3_512;
fn main() {
    let key = Key::generate();
    let secret = key.scalar;
    let public = key.compressed_ristretto().to_bytes();
    let alpha = [];
    let proof = Proof::new::<Sha3_512, Sha3_256>(&secret, &alpha);
    let beta = proof.hash::<Sha3_224>();
    println!("public {}", hex::encode(public));
    println!("beta {}", hex::encode(beta));
    println!("pi {}", hex::encode(proof.to_bytes()));
    println!(
        "verify {}",
        pea_vrf::validate_key(&public) && proof.verify::<Sha3_512, Sha3_256, Sha3_224>(&public, &alpha, &beta)
    );
}
