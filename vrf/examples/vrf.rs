use pea_key::Key;
use pea_vrf::{prove, verify};
fn main() {
    let key = Key::generate();
    let alpha = [0; 32];
    let (beta, pi) = prove(&alpha, &key.scalar);
    println!("{}", hex::encode(beta));
    println!("{}", verify(&alpha, key.ristretto_point(), beta, &pi));
}
