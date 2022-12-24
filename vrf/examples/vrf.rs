use pea_key::Key;
use pea_vrf::{prove, verify};
use sha3::Sha3_256;
use sha3::Sha3_512;
fn main() {
    let key = Key::generate();
    let mut alpha = [0; 32];
    for _ in 0..3 {
        for _ in 0..2 {
            let (beta, _) = prove::<Sha3_512, Sha3_256>(&alpha, &key.scalar);
            println!("{}", hex::encode(beta));
        }
        let (beta, pi) = prove::<Sha3_512, Sha3_256>(&alpha, &key.scalar);
        println!("{}", hex::encode(beta));
        println!("{}", verify::<Sha3_512, Sha3_256>(&alpha, &beta, key.ristretto_point(), &pi));
        alpha = beta;
    }
}
