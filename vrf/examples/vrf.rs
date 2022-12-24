use pea_key::Key;
use pea_vrf::{prove, verify};
fn main() {
    let key = Key::generate();
    let mut alpha = [0; 32];
    for _ in 0..3 {
        let (beta, _) = prove(&alpha, &key.scalar);
        println!("{}", hex::encode(beta));
        let (beta, pi) = prove(&alpha, &key.scalar);
        println!("{}", hex::encode(beta));
        println!("{}", verify(&alpha, beta, key.ristretto_point(), &pi));
        alpha = beta;
    }
}
