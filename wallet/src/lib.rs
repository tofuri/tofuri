pub mod cmd;
pub mod inquire;
use clap::Parser;
use colored::*;
use crossterm::event;
use crossterm::terminal;
use key::Key;
use reqwest::Url;
use std::error::Error;
use std::path::Path;
const INCORRECT: &str = "Incorrect passphrase";
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// API Endpoint
    #[clap(long, env = "API", default_value = "http://localhost:2021/")]
    pub api: Url,
}
pub fn decrypt(key: &mut Option<Key>, path: &Path) -> Result<bool, Box<dyn Error>> {
    println!("{}", path.to_string_lossy().green());
    let encrypted = key_store::read(path);
    loop {
        match {
            let pwd = inquire::pwd()?;
            let key = encryption::decrypt(&encrypted, pwd)
                .and_then(|secret_key_bytes| Key::from_slice(&secret_key_bytes).ok());
            if key.is_none() {
                println!("{}", INCORRECT.red())
            }
            key
        } {
            Some(x) => {
                *key = Some(x);
                return Ok(false);
            }
            None => continue,
        }
    }
}
pub fn press_any_key_to_continue() {
    println!("{}", "Press any key to continue...".magenta().italic());
    terminal::enable_raw_mode().unwrap();
    event::read().unwrap();
    terminal::disable_raw_mode().unwrap();
}
pub fn clear() {
    print!("\x1B[2J\x1B[1;1H");
}
