#![feature(test)]
extern crate test;
use test::Bencher;
use tofuri_key::Key;
#[bench]
fn sign(b: &mut Bencher) {
    let key = Key::generate();
    let hash: [u8; 32] = rand::random();
    b.iter(|| key.sign(&hash).unwrap());
}
#[bench]
fn recover(b: &mut Bencher) {
    let key = Key::generate();
    let hash = [0; 32];
    let signature_bytes = key.sign(&hash).unwrap();
    b.iter(|| Key::recover(&hash, &signature_bytes).unwrap());
}
#[bench]
#[cfg(feature = "vrf")]
fn prove(b: &mut Bencher) {
    let key = Key::generate();
    let alpha: [u8; 32] = rand::random();
    b.iter(|| key.vrf_prove(&alpha).unwrap());
}
#[bench]
#[cfg(feature = "vrf")]
fn proof_to_hash(b: &mut Bencher) {
    let key = Key::generate();
    let alpha: [u8; 32] = rand::random();
    let pi = key.vrf_prove(&alpha).unwrap();
    b.iter(|| Key::vrf_proof_to_hash(&pi).unwrap());
}
#[bench]
#[cfg(feature = "vrf")]
fn verify(b: &mut Bencher) {
    let key = Key::generate();
    let alpha: [u8; 32] = rand::random();
    let pi = key.vrf_prove(&alpha).unwrap();
    b.iter(|| Key::vrf_verify(&key.public_key_bytes(), &pi, &alpha));
}
