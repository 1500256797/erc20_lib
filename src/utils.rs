use anyhow::Error;
use ethers::core::types::{Address, H160, U256};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::json;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;

#[derive(Debug)]
pub enum BalanceError {
    Reqwest(reqwest::Error),
    ParsingError,
}
impl From<reqwest::Error> for BalanceError {
    fn from(err: reqwest::Error) -> Self {
        BalanceError::Reqwest(err)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenPair {
    reserve0: U256,
    reserve1: U256,
}

// get list of holders top x holders
pub async fn get_blances_wit_top(
    address: Address,
    holders: &Vec<H160>,
    block: u64,
    rpc_url: &str,
    top: usize,
) -> Result<HashMap<H160, String>, Error> {
    let mut token_holders: HashMap<H160, String> = HashMap::new();
    let mut new_balances: Vec<U256> = vec![];
    match get_erc20_balance_at_block(format!("{:#x}", address), holders, block, rpc_url).await {
        Ok(balance) => {
            new_balances = balance;
        }
        Err(err) => {
            eprintln!("ERROR_write_balances: {:?}", err);
        }
    }
    if address
        == "0xd27b7d42d24d8f7c1cf5c46ccd3b986c396fde17"
            .parse()
            .unwrap()
    {
        println!("parsing SLP");
        let mut total_supply: U256 = 0.into();
        let mut total_reserves = TokenPair {
            reserve0: 0.into(),
            reserve1: 0.into(),
        };

        // "0x0902f1ac"; // getReserves()
        match parse_reserves(&rpc_url, block, "0x0902f1ac").await {
            Ok(reserves) => {
                println!("Total reserves: {:?}", reserves);
                total_reserves = reserves;
            }
            Err(err) => {
                eprintln!("ERROR: {:?}", err);
            }
        }

        match parse_total_supply(&rpc_url, block, "0x18160ddd").await {
            Ok(supply) => {
                println!("Total supply: {:?}", supply);
                total_supply = supply;
            }
            Err(err) => {
                eprintln!("ERROR: {:?}", err);
            }
        }
        for (holder, balance) in holders.iter().zip(new_balances.iter()) {
            if balance.as_u128() == 0 {
                continue;
            }
            let precision = U256::exp10(18);

            let user_share = (balance * precision) / total_supply;
            println!("User share: {:?}", user_share);

            // Convert user's share to the amount of tokens in reserve1
            let user_reserve1 = (user_share * total_reserves.reserve1) / precision;

            println!(
                "{}: balance of SLP {} | balance of SDL {}",
                holder, balance, user_reserve1
            );
            token_holders.insert(*holder, user_reserve1.to_string());
        }
    } else {
        // set token holders with their balances
        for (holder, balance) in holders.iter().zip(new_balances.iter()) {
            if balance.as_u128() == 0 {
                continue;
            }

            token_holders.insert(*holder, balance.to_string());
        }

        // sort token holders by their balances
        let mut sorted_holders: Vec<(H160, U256)> = token_holders
            .iter()
            .map(|(k, v)| (*k, U256::from_dec_str(v).unwrap()))
            .collect();

        sorted_holders.sort_by(|a, b| b.1.cmp(&a.1));

        // get top x holders
        let top_holders: Vec<(H160, U256)> = sorted_holders.iter().take(top).cloned().collect();

        // set token holders with their balances
        token_holders = top_holders
            .iter()
            .map(|(k, v)| (*k, v.to_string()))
            .collect();
    }
    Ok(token_holders)
}

pub async fn write_balances(
    address: Address,
    holders: &Vec<H160>,
    file_name: String,
    block: u64,
    rpc_url: &str,
) -> Result<(), Error> {
    println!("Writing balances for {}", file_name);

    let mut token_holders: HashMap<H160, String> = HashMap::new();
    let mut new_balances: Vec<U256> = vec![];
    match get_erc20_balance_at_block(format!("{:#x}", address), holders, block, rpc_url).await {
        Ok(balance) => {
            new_balances = balance;
        }
        Err(err) => {
            eprintln!("ERROR_write_balances: {:?}", err);
        }
    }
    if address
        == "0xd27b7d42d24d8f7c1cf5c46ccd3b986c396fde17"
            .parse()
            .unwrap()
    {
        println!("parsing SLP");
        let mut total_supply: U256 = 0.into();
        let mut total_reserves = TokenPair {
            reserve0: 0.into(),
            reserve1: 0.into(),
        };

        // "0x0902f1ac"; // getReserves()
        match parse_reserves(&rpc_url, block, "0x0902f1ac").await {
            Ok(reserves) => {
                println!("Total reserves: {:?}", reserves);
                total_reserves = reserves;
            }
            Err(err) => {
                eprintln!("ERROR: {:?}", err);
            }
        }

        match parse_total_supply(&rpc_url, block, "0x18160ddd").await {
            Ok(supply) => {
                println!("Total supply: {:?}", supply);
                total_supply = supply;
            }
            Err(err) => {
                eprintln!("ERROR: {:?}", err);
            }
        }
        for (holder, balance) in holders.iter().zip(new_balances.iter()) {
            if balance.as_u128() == 0 {
                continue;
            }
            let precision = U256::exp10(18);

            let user_share = (balance * precision) / total_supply;
            println!("User share: {:?}", user_share);

            // Convert user's share to the amount of tokens in reserve1
            let user_reserve1 = (user_share * total_reserves.reserve1) / precision;

            println!(
                "{}: balance of SLP {} | balance of SDL {}",
                holder, balance, user_reserve1
            );
            token_holders.insert(*holder, user_reserve1.to_string());
        }
    } else {
        for (holder, balance) in holders.iter().zip(new_balances.iter()) {
            if balance.as_u128() == 0 {
                continue;
            }

            token_holders.insert(*holder, balance.to_string());
        }
    }

    let json_data = serde_json::to_string(&token_holders)?;
    let mut file = File::create(file_name)?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}

async fn get_erc20_balance_at_block(
    token_address: String,
    holder_addresses: &Vec<H160>,
    block: u64,
    rpc_url: &str,
) -> Result<Vec<U256>, BalanceError> {
    let string_holders: Vec<String> = holder_addresses
        .iter()
        .map(|h| format!("{:#x}", h))
        .collect();

    let requests: Vec<serde_json::Value> = string_holders
        .iter()
        .enumerate()
        .map(|(i, holder_address)| {
            json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "eth_call",
                "params": [
                    {
                        "to": token_address,
                        "data": format!(
                            "0x70a08231000000000000000000000000{}",
                            &holder_address[2..]
                        )
                    },
                    format!("0x{:x}", block)
                ]
            })
        })
        .collect();

    let client = reqwest::Client::new();
    let response: Vec<serde_json::Value> = client
        .post(rpc_url)
        .json(&requests)
        .send()
        .await?
        .json()
        .await?;

    let balances: Result<Vec<U256>, BalanceError> = response
        .into_iter()
        .map(|r| {
            let balance_hex = r["result"].as_str().ok_or(BalanceError::ParsingError)?;
            U256::from_str(balance_hex).map_err(|_| BalanceError::ParsingError)
        })
        .collect();

    balances
}

async fn _get_mapping_total(
    token_address: String,
    holder_addresses: &Vec<H160>,
    block: u64,
    rpc_url: &str,
) -> Result<Vec<U256>, BalanceError> {
    let string_holders: Vec<String> = holder_addresses
        .iter()
        .map(|h| format!("{:#x}", h))
        .collect();

    let requests: Vec<serde_json::Value> = string_holders
        .iter()
        .enumerate()
        .map(|(i, holder_address)| {
            json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "eth_call",
                "params": [
                    {
                        "to": token_address,
                        "data": format!(
                            "0x93f1a40b0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000{}",
                            &holder_address[2..]
                        )
                    },
                    format!("0x{:x}", block)
                ]
            })
        })
        .collect();

    let client = reqwest::Client::new();
    let response: Vec<serde_json::Value> = client
        .post(rpc_url)
        .json(&requests)
        .send()
        .await?
        .json()
        .await?;

    let amounts: Result<Vec<U256>, BalanceError> = response
        .into_iter()
        .map(|r| {
            let res = r["result"].as_str().ok_or(BalanceError::ParsingError)?;
            let amount_hex = &res[0..64];
            U256::from_str(amount_hex).map_err(|_| BalanceError::ParsingError)
        })
        .collect();
    amounts
}

async fn query_contract(
    rpc_url: &str,
    block: u64,
    func_sig: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let data = format!("{}", func_sig);
    let call_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [
            {
                "to": "0xd27b7d42d24d8f7c1cf5c46ccd3b986c396fde17",
                "data": data
            },
            format!("0x{:x}", block)
        ]
    });

    let client = reqwest::Client::new();
    let response: serde_json::Value = client
        .post(rpc_url)
        .json(&call_request)
        .send()
        .await?
        .json()
        .await?;

    let res = response["result"].as_str().unwrap();
    Ok(res.to_string())
}

async fn parse_reserves(
    rpc_url: &str,
    block: u64,
    func_sig: &str,
) -> Result<TokenPair, Box<dyn std::error::Error>> {
    match query_contract(rpc_url, block, func_sig).await {
        Ok(res) => {
            let reserve0 = &res[2..66];
            let reserve1 = &res[66..130];
            let r0 = U256::from_str(reserve0).map_err(|_| BalanceError::ParsingError);
            let r1 = U256::from_str(reserve1).map_err(|_| BalanceError::ParsingError);

            let token_pair = TokenPair {
                reserve0: r0.unwrap(),
                reserve1: r1.unwrap(),
            };

            Ok(token_pair)
        }
        Err(e) => Err(e),
    }
}

async fn parse_total_supply(
    rpc_url: &str,
    block: u64,
    func_sig: &str,
) -> Result<U256, Box<dyn std::error::Error>> {
    match query_contract(rpc_url, block, func_sig).await {
        Ok(res) => {
            let supply_hex = &res[2..66];

            let supply = U256::from_str(supply_hex).map_err(|_| BalanceError::ParsingError);

            Ok(supply.unwrap())
        }
        Err(e) => Err(e),
    }
}
