use super::util::{self, print};
use argon2::password_hash::rand_core::RngCore;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305,
};
use colored::*;
use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use std::{error::Error, fs::File, io::prelude::*, path::Path};
pub struct Wallet {
    pub keypair: Keypair,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}
impl Wallet {
    pub fn new() -> Wallet {
        Wallet {
            keypair: util::keygen(),
            salt: vec![],
            nonce: vec![],
            ciphertext: vec![],
        }
    }
    pub fn import() -> Wallet {
        let wallet;
        loop {
            if let Ok(w) = Wallet::import_attempt() {
                wallet = w;
                break;
            } else {
                println!("{}", "No key available with this passphrase.".red());
                command::press_any_key_to_continue();
            }
        }
        wallet
    }
    pub fn import_attempt() -> Result<Wallet, Box<dyn Error>> {
        let encrypted_secret_key_bytes = match Wallet::read(Wallet::default_path()) {
            Ok(secret_key_bytes) => secret_key_bytes,
            Err(err) => {
                util::print::err(err);
                println!("{}", "Generating new wallet...".yellow());
                let mut wallet = Wallet::new();
                wallet.export()?;
                return Ok(wallet);
            }
        };
        let salt = &encrypted_secret_key_bytes[..32];
        let nonce = &encrypted_secret_key_bytes[32..44];
        let ciphertext = &encrypted_secret_key_bytes[44..];
        print::encrypted_secret_key_bytes(salt, nonce, ciphertext);
        let secret_key = SecretKey::from_bytes(&Wallet::decrypt(salt, nonce, ciphertext)?)?;
        let public_key: PublicKey = (&secret_key).into();
        Ok(Wallet {
            keypair: Keypair {
                secret: secret_key,
                public: public_key,
            },
            salt: salt.to_vec(),
            nonce: nonce.to_vec(),
            ciphertext: ciphertext.to_vec(),
        })
    }
    pub fn export(&mut self) -> Result<(), Box<dyn Error>> {
        let (salt, nonce, ciphertext) = Wallet::encrypt(self.keypair.secret.as_bytes())?;
        self.salt = salt.to_vec();
        self.nonce = nonce.to_vec();
        self.ciphertext = ciphertext.to_vec();
        print::encrypted_secret_key_bytes(&salt, &nonce, &ciphertext);
        Wallet::write(
            Wallet::default_path(),
            &[salt.to_vec(), nonce.to_vec(), ciphertext].concat(),
        )?;
        Ok(())
    }
    fn read(path: impl AsRef<Path>) -> Result<[u8; 92], Box<dyn Error>> {
        let mut file = File::open(path)?;
        let mut buf = [0; 92];
        file.read(&mut buf)?;
        Ok(buf)
    }
    fn write(path: &Path, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(path)?;
        file.write_all(buf)?;
        Ok(())
    }
    fn default_path() -> &'static Path {
        Path::new("./encrypted_secret_key_bytes")
    }
    pub fn address(&self) -> String {
        address::encode(&self.keypair.public.as_bytes())
    }
    pub fn key(&self) -> String {
        key::encode(&self.keypair.secret)
    }
    pub fn encrypt(plaintext: &[u8]) -> Result<([u8; 32], [u8; 12], Vec<u8>), Box<dyn Error>> {
        let passphrase = command::new_passphrase();
        let rng = &mut OsRng;
        let mut salt = [0; 32];
        rng.fill_bytes(&mut salt);
        let key = kdf::derive(passphrase.as_bytes(), &salt);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext).unwrap();
        Ok((salt, nonce.into(), ciphertext))
    }
    pub fn decrypt(
        salt: &[u8],
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let passphrase = command::passphrase();
        let key = kdf::derive(passphrase.as_bytes(), salt);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
        match cipher.decrypt(nonce.into(), ciphertext) {
            Ok(plaintext) => Ok(plaintext),
            Err(_) => Err("Wrong passphrase".into()),
        }
    }
}
pub mod command {
    use super::{address, util::print, Wallet};
    use crate::{stake::Stake, transaction::Transaction};
    use colored::*;
    use inquire::{
        validator::Validation, Confirm, CustomType, Password, PasswordDisplayMode, Select,
    };
    use std::{
        collections::HashMap,
        error::Error,
        io::{stdin, stdout, Write},
        process,
    };
    use termion::{input::TermRead, raw::IntoRawMode};
    pub async fn main(wallet: &Wallet, api: &str) -> Result<(), Box<dyn Error>> {
        match Select::new(
            ">>",
            vec![
                "address",
                "key",
                "data",
                "balance",
                "height",
                "transaction",
                "stake",
                "ip",
                "validator",
                "exit",
            ],
        )
        .prompt()
        .unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }) {
            "address" => address(&wallet),
            "key" => key(&wallet),
            "data" => data(&wallet),
            "balance" => balance(api, &wallet.address()).await?,
            "height" => height(api).await?,
            "transaction" => transaction(api, &wallet).await?,
            "stake" => stake(api, &wallet).await?,
            "ip" => ip().await?,
            "validator" => validator(api).await?,
            "exit" => exit(),
            _ => {}
        }
        Ok(())
    }
    pub fn press_any_key_to_continue() {
        println!("Press any key to continue...");
        let mut stdout = stdout().into_raw_mode().unwrap();
        stdout.flush().unwrap();
        stdin().events().next();
        print::clear();
    }
    pub async fn validator(api: &str) -> Result<(), Box<dyn Error>> {
        let info = reqwest::get(api).await?.text().await?;
        println!("\n{}\n", info.green());
        Ok(())
    }
    pub async fn balance(api: &str, address: &str) -> Result<(), Box<dyn Error>> {
        let balance = reqwest::get(format!("{}/balance/{}", api, address))
            .await?
            .json::<u64>()
            .await?;
        let balance_staked = reqwest::get(format!("{}/balance_staked/{}", api, address))
            .await?
            .json::<u64>()
            .await?;
        println!(
            "Account balance: {}, locked: {}.",
            balance.to_string().yellow(),
            balance_staked.to_string().yellow()
        );
        Ok(())
    }
    pub async fn height(api: &str) -> Result<(), Box<dyn Error>> {
        let balance = reqwest::get(format!("{}/height", api))
            .await?
            .json::<u64>()
            .await?;
        println!("Latest block height is {}.", balance.to_string().yellow());
        Ok(())
    }
    pub async fn transaction(api: &str, wallet: &Wallet) -> Result<(), Box<dyn Error>> {
        let address = CustomType::<String>::new("address >>")
            .with_error_message("Please enter a valid address")
            .with_help_message("Type the hex encoded address with 0x as prefix")
            .with_parser(&|x| match address::decode(x) {
                Ok(y) => Ok(address::encode(&y)),
                Err(_) => Err(()),
            })
            .prompt()?;
        let amount = (CustomType::<f64>::new("amount >>")
            .with_formatter(&|i| format!("{:.8} C", i))
            .with_error_message("Please type a valid number")
            .with_help_message("Type the amount in C using a decimal point as a separator")
            .prompt()?
            * 10f64.powi(8)) as u64;
        let fee = CustomType::<u64>::new("fee >>")
            .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
            .with_error_message("Please type a valid number")
            .with_help_message("Type the amount in satoshis using a decimal point as a separator")
            .prompt()?;
        if !Confirm::new("send >>").prompt()? {
            return Ok(());
        }
        let mut transaction = Transaction::new(address::decode(&address)?, amount, fee);
        transaction.sign(&wallet.keypair);
        let client = reqwest::Client::new();
        let res: usize = client
            .post(format!("{}/transaction", api))
            .body(hex::encode(bincode::serialize(&transaction)?))
            .send()
            .await?
            .json()
            .await?;
        println!(
            "{} {}",
            if res == 1 {
                "Transaction successfully sent!".green()
            } else if res == 0 {
                "Transaction failed to send!".red()
            } else {
                "Unexpected status".cyan()
            },
            hex::encode(&transaction.hash())
        );
        Ok(())
    }
    pub async fn stake(api: &str, wallet: &Wallet) -> Result<(), Box<dyn Error>> {
        let deposit = match Select::new(">>", vec!["deposit", "withdraw"])
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            }) {
            "deposit" => true,
            "withdraw" => false,
            _ => false,
        };
        let amount = (CustomType::<f64>::new("amount >>")
            .with_formatter(&|i| format!("{:.8} C", i))
            .with_error_message("Please type a valid number")
            .with_help_message("Type the amount in C using a decimal point as a separator")
            .prompt()?
            * 10f64.powi(8)) as u64;
        let fee = CustomType::<u64>::new("fee >>")
            .with_formatter(&|i| format!("{} {}", i, if i == 1 { "satoshi" } else { "satoshis" }))
            .with_error_message("Please type a valid number")
            .with_help_message("Type the amount in satoshis using a decimal point as a separator")
            .prompt()?;
        if !Confirm::new("send >>").prompt()? {
            return Ok(());
        }
        let mut stake = Stake::new(deposit, amount as u64, fee);
        stake.sign(&wallet.keypair);
        let client = reqwest::Client::new();
        let res: usize = client
            .post(format!("{}/stake", api))
            .body(hex::encode(bincode::serialize(&stake)?))
            .send()
            .await?
            .json()
            .await?;
        println!(
            "{} {}",
            if res == 1 {
                "Stake successfully sent!".green()
            } else if res == 0 {
                "Stake failed to send!".red()
            } else {
                "Unexpected status".cyan()
            },
            hex::encode(&stake.hash())
        );
        Ok(())
    }
    pub async fn ip() -> Result<(), Box<dyn Error>> {
        let resp = reqwest::get("https://httpbin.org/ip")
            .await?
            .json::<HashMap<String, String>>()
            .await?;
        if let Some(origin) = resp.get("origin") {
            println!("{}", origin.yellow());
        }
        Ok(())
    }
    pub fn address(wallet: &Wallet) {
        println!("{}", wallet.address().green());
    }
    pub fn key(wallet: &Wallet) {
        println!("{}", wallet.key().red());
    }
    pub fn data(wallet: &Wallet) {
        print::encrypted_secret_key_bytes(&wallet.salt, &wallet.nonce, &wallet.ciphertext);
    }
    pub fn exit() {
        process::exit(0);
    }
    pub fn passphrase() -> String {
        Password::new("Enter passphrase:")
            .with_display_toggle_enabled()
            .with_display_mode(PasswordDisplayMode::Masked)
            .with_formatter(&|_| String::from("Decrypting..."))
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            })
    }
    pub fn new_passphrase() -> String {
        let passphrase = Password::new("New passphrase:")
            .with_display_toggle_enabled()
            .with_display_mode(PasswordDisplayMode::Masked)
            .with_formatter(&|input| {
                let entropy = zxcvbn::zxcvbn(input, &[]).unwrap();
                format!(
                    "{}. Cracked after {} at 10 guesses per second.",
                    match entropy.score() {
                        0 => "Extremely weak",
                        1 => "Very weak",
                        2 => "Weak",
                        3 => "Strong",
                        4 => "Very strong",
                        _ => "",
                    },
                    entropy.crack_times().online_no_throttling_10_per_second(),
                )
            })
            .with_help_message("It is recommended to generate a new one only for this purpose")
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            });
        let validator = move |input: &str| {
            if passphrase != input {
                Ok(Validation::Invalid("Passphrase does not match.".into()))
            } else {
                Ok(Validation::Valid)
            }
        };
        Password::new("Confirm new passphrase:")
            .with_display_toggle_enabled()
            .with_display_mode(PasswordDisplayMode::Masked)
            .with_validator(validator)
            .with_formatter(&|_| String::from("Encrypting..."))
            .prompt()
            .unwrap_or_else(|err| {
                println!("{}", err.to_string().red());
                process::exit(0)
            })
    }
}
pub mod address {
    use crate::{constants::PREFIX_ADDRESS, util};
    use std::error::Error;
    fn checksum(decoded: &[u8]) -> [u8; 4] {
        util::hash(decoded).get(0..4).unwrap().try_into().unwrap()
    }
    pub fn encode(public_key: &[u8; 32]) -> String {
        [
            PREFIX_ADDRESS,
            &hex::encode(public_key),
            &hex::encode(checksum(public_key)),
        ]
        .concat()
    }
    pub fn decode(address: &str) -> Result<[u8; 32], Box<dyn Error>> {
        let decoded = hex::decode(address.replacen(PREFIX_ADDRESS, "", 1))?;
        let address: [u8; 32] = decoded
            .get(0..32)
            .ok_or("Invalid address")?
            .try_into()
            .unwrap();
        if checksum(&address) == decoded.get(32..).ok_or("Invalid checksum")? {
            Ok(address)
        } else {
            Err("checksum mismatch".into())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use test::Bencher;
        #[test]
        fn test_cecksum() {
            assert_eq!(vec![0x60, 0x7b, 0x1a, 0xff], checksum(&vec![0; 20]));
        }
        #[bench]
        fn bench_cecksum(b: &mut Bencher) {
            b.iter(|| checksum(&vec![0; 20]));
        }
    }
}
pub mod key {
    use crate::{constants::PREFIX_ADDRESS_KEY, util::hash};
    use std::error::Error;
    fn checksum(decoded: &[u8]) -> [u8; 4] {
        hash(decoded).get(1..5).unwrap().try_into().unwrap()
    }
    pub fn encode(secret_key: &ed25519_dalek::SecretKey) -> String {
        [
            PREFIX_ADDRESS_KEY,
            &hex::encode(secret_key),
            &hex::encode(checksum(secret_key.as_bytes())),
        ]
        .concat()
    }
    pub fn decode(secret_key: &str) -> Result<[u8; 32], Box<dyn Error>> {
        let decoded = hex::decode(secret_key.replacen(PREFIX_ADDRESS_KEY, "", 1))?;
        println!("{:?}", decoded);
        let secret_key: [u8; 32] = decoded
            .get(0..32)
            .ok_or("Invalid address")?
            .try_into()
            .unwrap();
        if checksum(&secret_key) == decoded.get(32..).ok_or("Invalid checksum")? {
            Ok(secret_key)
        } else {
            Err("checksum mismatch".into())
        }
    }
}
pub mod kdf {
    use argon2::{Algorithm, Argon2, Params, ParamsBuilder, Version};
    fn params() -> Params {
        let mut builder = ParamsBuilder::new();
        builder.m_cost(65536).unwrap();
        builder.t_cost(1).unwrap();
        builder.p_cost(1).unwrap();
        builder.params().unwrap()
    }
    pub fn derive(password: &[u8], salt: &[u8]) -> [u8; 32] {
        let ctx = Argon2::new(Algorithm::Argon2id, Version::V0x13, params());
        let mut out = [0u8; 32];
        ctx.hash_password_into(password, salt, &mut out).unwrap();
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[bench]
    fn bench_kdf_derive(b: &mut Bencher) {
        b.iter(|| kdf::derive("test".as_bytes(), &[0; 32]));
    }
}
