use anyhow::anyhow;
use anyhow::{Ok, Result};
use ethers::{
    core::types::{Address, Filter},
    providers::{Http, Middleware, Provider},
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ethers::types::H160;

pub mod utils;

pub async fn get_erc20_token_top_holders_balance(
    contract_address: Address,
    from_block: u64,
    to_block: u64,
    rpc_url: &str,
    batch_size: u64,
    top: usize,
) -> Result<HashMap<H160, String>> {
    // check params from_block and to_block
    if from_block > to_block {
        return Err(anyhow!("from_block must be less than to_block"));
    }
    // batch size must be greater than 0 and less than 10000
    if batch_size == 0 || batch_size > 10000 {
        return Err(anyhow!(
            "batch_size must be greater than 0 and less than 10000"
        ));
    }

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let client = Arc::new(provider);
    let mut from_block = from_block;
    let mut token_holders: HashSet<Address> = HashSet::new();
    while from_block <= to_block {
        let to_block = (from_block + batch_size).min(to_block);
        println!("Fetching logs from block {} to {}", from_block, to_block);

        let filter = Filter::new()
            .address(contract_address)
            .event("Transfer(address,address,uint256)")
            .from_block(from_block)
            .to_block(to_block);

        let logs = client.get_logs(&filter).await?;

        for log in logs.iter() {
            let from = Address::from(log.topics[1]);
            let to = Address::from(log.topics[2]);
            token_holders.insert(from);
            token_holders.insert(to);
        }

        from_block = to_block + 1;
    }
    // get all token holders in the history
    println!("Done capturing token holders");
    let token_holders: Vec<Address> = token_holders.into_iter().collect();
    let res = utils::get_blances_wit_top(contract_address, &token_holders, to_block, rpc_url, top)
        .await
        .map_err(|e| eprintln!("Error: {:?}", e))
        .unwrap();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[tokio::test]
    async fn test_get_erc20_holders_balance() -> Result<()> {
        let contract_address =
            Address::from_str("0x7a58c0be72be218b41c608b7fe7c5bb630736c71").unwrap();
        let from_block = 20392228;
        let to_block = 20419528;
        let rpc_url = "https://eth-mainnet.g.alchemy.com/v2/d1Oxxxxxxxxxxxxxxxk";
        let batch_size = 10000;
        get_erc20_token_top_holders_balance(
            contract_address,
            from_block,
            to_block,
            rpc_url,
            batch_size,
            100,
        )
        .await;

        Ok(())
    }
}
