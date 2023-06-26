use crate::clear;
use crate::decrypt;
use crate::inquire;
use crate::inquire::GENERATE;
use crate::inquire::IMPORT;
use ::inquire::Confirm;
use ::inquire::Select;
use address::public;
use address::secret;
use api::BlockHex;
use api::Root;
use api::StakeHex;
use api::TransactionHex;
use chrono::Utc;
use colored::*;
use key::Key;
use key_store::DEFAULT_PATH;
use key_store::EXTENSION;
use rand::rngs::OsRng;
use reqwest::Client;
use std::process;
pub async fn select(client: &Client, api: &str, key: &mut Option<Key>) -> bool {
    let mut vec = vec!["Wallet", "Search", "Height", "API", "Exit"];
    if key.is_some() {
        let mut v = vec!["Address", "Balance", "Send", "Stake", "Secret"];
        v.append(&mut vec);
        vec = v;
    };
    match Select::new(">>", vec).prompt().unwrap_or_else(|err| {
        println!("{}", err.to_string().red());
        process::exit(0)
    }) {
        "Wallet" => wallet(key),
        "Search" => search(client, api).await,
        "Height" => height(client, api).await,
        "API" => root(client, api).await,
        "Address" => address(&key.as_ref().unwrap()),
        "Balance" => balance(client, api, &key.as_ref().unwrap()).await,
        "Send" => transaction(client, api, &key.as_ref().unwrap()).await,
        "Stake" => stake(client, api, &key.as_ref().unwrap()).await,
        "Secret" => view_secret(&key.as_ref().unwrap()),
        _ => process::exit(0),
    }
}
fn wallet(key: &mut Option<Key>) -> bool {
    let mut filename = crate::inquire::select().unwrap();
    let res = if filename.as_str() == *GENERATE {
        Some(Key::generate())
    } else if filename.as_str() == *IMPORT {
        Some(inquire::import_new().unwrap())
    } else {
        None
    };
    if let Some(key) = res {
        if !inquire::save_new() {
            return true;
        }
        filename = inquire::name_new().unwrap();
        let pwd = crate::inquire::pwd_new();
        let rng = &mut OsRng;
        key_store::write(rng, &key, &filename, &pwd);
    }
    let mut path = DEFAULT_PATH.join(filename);
    path.set_extension(EXTENSION);
    clear();
    decrypt(key, &path);
    false
}
async fn root(client: &Client, api: &str) -> bool {
    let root: Root = client
        .get(api.to_string())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("{root:#?}");
    true
}
async fn balance(client: &Client, api: &str, key: &Key) -> bool {
    let address = public::encode(&key.address_bytes());
    let balance: String = client
        .get(format!("{}balance/{}", api.to_string(), address))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let staked: String = client
        .get(format!("{}staked/{}", api.to_string(), address))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!(
        "Account balance: {}, staked: {}",
        balance.to_string().yellow(),
        staked.yellow()
    );
    true
}
async fn height(client: &Client, api: &str) -> bool {
    let height: usize = client
        .get(format!("{}height", api.to_string()))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!("Latest block height is {}.", height.to_string().yellow());
    true
}
async fn transaction(client: &Client, api: &str, key: &Key) -> bool {
    let address = inquire::address();
    let amount = inquire::amount();
    let fee = inquire::fee();
    if !match Confirm::new("Send?").prompt() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0)
        }
    } {
        return false;
    }
    let transaction = transaction::Transaction::sign(
        public::decode(&address).unwrap(),
        amount,
        fee,
        Utc::now().timestamp() as u32,
        key,
    )
    .unwrap();
    println!("[u8; 32]: {}", hex::encode(transaction.hash()).cyan());
    let transaction_hex: TransactionHex = transaction.try_into().unwrap();
    let res: String = client
        .post(format!("{}transaction", api.to_string()))
        .json(&transaction_hex)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!(
        "{}",
        if res == "success" {
            res.green()
        } else {
            res.red()
        }
    );
    true
}
async fn stake(client: &Client, api: &str, key: &Key) -> bool {
    let deposit = inquire::deposit();
    let amount = inquire::amount();
    let fee = inquire::fee();
    let send = inquire::confirm_send();
    if !send {
        return false;
    }
    let stake =
        stake::Stake::sign(deposit, amount, fee, Utc::now().timestamp() as u32, key).unwrap();
    println!("[u8; 32]: {}", hex::encode(stake.hash()).cyan());
    let stake_hex: StakeHex = stake.try_into().unwrap();
    let res: String = client
        .post(format!("{}stake", api.to_string()))
        .json(&stake_hex)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    println!(
        "{}",
        if res == "success" {
            res.green()
        } else {
            res.red()
        }
    );
    true
}
async fn search(client: &Client, api: &str) -> bool {
    let search = inquire::search();
    if public::decode(&search).is_ok() {
        let balance: String = client
            .get(format!("{}balance/{}", api.to_string(), search))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let staked: String = client
            .get(format!("{}staked/{}", api.to_string(), search))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        println!(
            "Address found\nAccount balance: {}, staked: {}",
            balance.to_string().yellow(),
            staked.yellow()
        );
        return true;
    } else if search.len() == 64 {
        if let Ok(res) = client
            .get(format!("{}block/{}", api.to_string(), search))
            .send()
            .await
        {
            let block: BlockHex = res.json().await.unwrap();
            println!("Block found\n{block:?}");
        } else if let Ok(res) = client
            .get(format!("{}/transaction/{}", api.to_string(), search))
            .send()
            .await
        {
            let transaction: TransactionHex = res.json().await.unwrap();
            println!("Transaction found\n{transaction:?}");
        } else if let Ok(res) = client
            .get(format!("{}stake/{}", api.to_string(), search))
            .send()
            .await
        {
            let stake: StakeHex = res.json().await.unwrap();
            println!("Stake found\n{stake:?}");
        }
    } else if search.parse::<usize>().is_ok() {
        if let Ok(res) = client
            .get(format!("{}hash/{}", api.to_string(), search))
            .send()
            .await
        {
            let hash: String = res.json().await.unwrap();
            if let Ok(res) = client
                .get(format!("{}block/{}", api.to_string(), hash))
                .send()
                .await
            {
                let block: BlockHex = res.json().await.unwrap();
                println!("Block found\n{block:?}");
            }
        }
    } else {
        println!("{}", "Nothing found".red());
    }
    true
}
fn address(key: &Key) -> bool {
    println!("{}", public::encode(&key.address_bytes()).green());
    true
}
fn view_secret(key: &Key) -> bool {
    println!("{}", "Are you being watched?".yellow());
    println!("{}", "Never share your secret key!".yellow());
    println!(
        "{}",
        "Anyone who has it can access your funds from anywhere.".italic()
    );
    println!("{}", "View in private with no cameras around.".italic());
    if match Confirm::new("View secret key?").prompt() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err.to_string().red());
            process::exit(0)
        }
    } {
        println!("{}", secret::encode(&key.secret_key_bytes()).red());
    }
    true
}
