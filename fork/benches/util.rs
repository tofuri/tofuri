#![feature(test)]
extern crate test;
use sha2::Digest;
use sha2::Sha256;
use test::Bencher;
#[bench]
fn hash(b: &mut Bencher) {
    b.iter(|| {
        let mut hasher = Sha256::new();
        hasher.update([0; 32]);
        hasher.finalize();
    });
}
#[bench]
fn u256(b: &mut Bencher) {
    b.iter(|| tofuri_fork::u256(&[0xff; 32]));
}
#[bench]
fn random(b: &mut Bencher) {
    let mut hasher = Sha256::new();
    hasher.update([0; 32]);
    let beta: [u8; 32] = hasher.finalize().into();
    b.iter(|| tofuri_fork::random(&beta, 0, 10));
}
