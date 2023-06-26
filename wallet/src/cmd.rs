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
use std::error::Error;
use std::process;
pub async fn select(
    client: &Client,
    api: &str,
    key: &mut Option<Key>,
) -> Result<bool, Box<dyn Error>> {
    let mut vec = vec!["Wallet", "Search", "Height", "API", "Exit"];
    if key.is_some() {
        let mut v = vec!["Address", "Balance", "Send", "Stake", "Secret"];
        v.append(&mut vec);
        vec = v;
    };
    Ok(
        match Select::new(">>", vec).prompt().unwrap_or_else(|err| {
            println!("{}", err.to_string().red());
            process::exit(0)
        }) {
            "Wallet" => wallet(key)?,
            "Search" => search(client, api).await?,
            "Height" => height(client, api).await?,
            "API" => root(client, api).await?,
            "Address" => address(&key.as_ref().unwrap()),
            "Balance" => balance(client, api, &key.as_ref().unwrap()).await?,
            "Send" => transaction(client, api, &key.as_ref().unwrap()).await?,
            "Stake" => stake(client, api, &key.as_ref().unwrap()).await?,
            "Secret" => view_secret(&key.as_ref().unwrap())?,
            _ => process::exit(0),
        },
    )
}
fn wallet(key: &mut Option<Key>) -> Result<bool, Box<dyn Error>> {
    let mut filename = inquire::select().unwrap();
    let res = if filename.as_str() == *GENERATE {
        Some(Key::generate())
    } else if filename.as_str() == *IMPORT {
        Some(inquire::import_new().unwrap())
    } else {
        None
    };
    if let Some(key) = res {
        if !inquire::save_new() {
            return Ok(true);
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
    Ok(false)
}
async fn root(client: &Client, api: &str) -> Result<bool, Box<dyn Error>> {
    let root: Root = client.get(api.to_string()).send().await?.json().await?;
    println!("{root:#?}");
    Ok(true)
}
async fn balance(client: &Client, api: &str, key: &Key) -> Result<bool, Box<dyn Error>> {
    let address = public::encode(&key.address_bytes());
    let balance: String = client
        .get(format!("{}balance/{}", api.to_string(), address))
        .send()
        .await?
        .json()
        .await?;
    let staked: String = client
        .get(format!("{}staked/{}", api.to_string(), address))
        .send()
        .await?
        .json()
        .await?;
    println!(
        "Account balance: {}, staked: {}",
        balance.to_string().yellow(),
        staked.yellow()
    );
    Ok(true)
}
async fn height(client: &Client, api: &str) -> Result<bool, Box<dyn Error>> {
    let height: usize = client
        .get(format!("{}height", api.to_string()))
        .send()
        .await?
        .json()
        .await?;
    println!("Latest block height is {}.", height.to_string().yellow());
    Ok(true)
}
async fn transaction(client: &Client, api: &str, key: &Key) -> Result<bool, Box<dyn Error>> {
    let address = inquire::address();
    let amount = inquire::amount();
    let fee = inquire::fee();
    if !Confirm::new("Send?").prompt()? {
        return Ok(false);
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
    Ok(true)
}
async fn stake(client: &Client, api: &str, key: &Key) -> Result<bool, Box<dyn Error>> {
    let deposit = inquire::deposit();
    let amount = inquire::amount();
    let fee = inquire::fee();
    let send = inquire::confirm_send();
    if !send {
        return Ok(false);
    }
    let stake =
        stake::Stake::sign(deposit, amount, fee, Utc::now().timestamp() as u32, key).unwrap();
    println!("[u8; 32]: {}", hex::encode(stake.hash()).cyan());
    let stake_hex: StakeHex = stake.try_into().unwrap();
    let res: String = client
        .post(format!("{}stake", api.to_string()))
        .json(&stake_hex)
        .send()
        .await?
        .json()
        .await?;
    println!(
        "{}",
        if res == "success" {
            res.green()
        } else {
            res.red()
        }
    );
    Ok(true)
}
async fn search(client: &Client, api: &str) -> Result<bool, Box<dyn Error>> {
    let search = inquire::search();
    if public::decode(&search).is_ok() {
        let balance: String = client
            .get(format!("{}balance/{}", api.to_string(), search))
            .send()
            .await?
            .json()
            .await?;
        let staked: String = client
            .get(format!("{}staked/{}", api.to_string(), search))
            .send()
            .await?
            .json()
            .await?;
        println!(
            "Address found\nAccount balance: {}, staked: {}",
            balance.to_string().yellow(),
            staked.yellow()
        );
        return Ok(true);
    } else if search.len() == 64 {
        if let Ok(res) = client
            .get(format!("{}block/{}", api.to_string(), search))
            .send()
            .await
        {
            let block: BlockHex = res.json().await?;
            println!("Block found\n{block:?}");
        } else if let Ok(res) = client
            .get(format!("{}/transaction/{}", api.to_string(), search))
            .send()
            .await
        {
            let transaction: TransactionHex = res.json().await?;
            println!("Transaction found\n{transaction:?}");
        } else if let Ok(res) = client
            .get(format!("{}stake/{}", api.to_string(), search))
            .send()
            .await
        {
            let stake: StakeHex = res.json().await?;
            println!("Stake found\n{stake:?}");
        }
    } else if search.parse::<usize>().is_ok() {
        if let Ok(res) = client
            .get(format!("{}hash/{}", api.to_string(), search))
            .send()
            .await
        {
            let hash: String = res.json().await?;
            if let Ok(res) = client
                .get(format!("{}block/{}", api.to_string(), hash))
                .send()
                .await
            {
                let block: BlockHex = res.json().await?;
                println!("Block found\n{block:?}");
            }
        }
    } else {
        println!("{}", "Nothing found".red());
    }
    Ok(true)
}
fn address(key: &Key) -> bool {
    println!("{}", public::encode(&key.address_bytes()).green());
    true
}
fn view_secret(key: &Key) -> Result<bool, Box<dyn Error>> {
    println!("{}", "Are you being watched?".yellow());
    println!("{}", "Never share your secret key!".yellow());
    println!(
        "{}",
        "Anyone who has it can access your funds from anywhere.".italic()
    );
    println!("{}", "View in private with no cameras around.".italic());
    if Confirm::new("View secret key?").prompt()? {
        println!("{}", secret::encode(&key.secret_key_bytes()).red());
    }
    Ok(true)
}
