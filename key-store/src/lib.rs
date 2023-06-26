use key::Key;
use lazy_static::lazy_static;
use rand_core::CryptoRngCore;
use std::fs::read_dir;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
pub const EXTENSION: &str = "tofuri";
lazy_static! {
    pub static ref DEFAULT_PATH: &'static Path = Path::new("./tofuri-wallet");
}
pub fn read(path: impl AsRef<Path>) -> [u8; 92] {
    let mut file = File::open(path).unwrap();
    let mut bytes = [0; 184];
    file.read_exact(&mut bytes).unwrap();
    let vec = hex::decode(bytes).unwrap();
    vec.try_into().unwrap()
}
pub fn write(rng: &mut impl CryptoRngCore, key: &Key, filename: &str, pwd: &str) {
    let encrypted = encryption::encrypt(rng, key.secret_key_bytes(), pwd);
    let mut path = DEFAULT_PATH.join(filename);
    path.set_extension(EXTENSION);
    let mut file = File::create(path).unwrap();
    file.write_all(hex::encode(encrypted).as_bytes()).unwrap();
}
pub fn filenames() -> Vec<String> {
    let mut filenames = vec![];
    if !DEFAULT_PATH.exists() {
        return filenames;
    }
    for entry in read_dir(*DEFAULT_PATH).unwrap() {
        filenames.push(
            entry
                .unwrap()
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        );
    }
    filenames
}
