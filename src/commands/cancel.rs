use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::config;
use crate::signing;

pub async fn run(order_id: &str, json_output: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;

    // Order ID format: "ASSET:OID" e.g. "BTC:91490942"
    let parts: Vec<&str> = order_id.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Order ID format: SYMBOL:OID (e.g. BTC:91490942)\n\
             Use `fintool orders` to list your open orders."
        );
    }
    let asset = parts[0].to_uppercase();
    let oid: u64 = parts[1].parse().context("Invalid order ID number")?;

    if !json_output {
        println!();
        println!("  🗑️ Cancelling order");
        println!("  Symbol:   {}", asset.cyan());
        println!("  Order ID: {}", oid);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = signing::cancel_order(&asset, oid).await?;

    let response = json!({
        "action": "cancel",
        "symbol": asset,
        "orderId": oid,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match result {
            hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(data) => {
                println!("  ✅ Order cancelled!");
                println!("  Response: {:?}", data);
            }
            hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
                println!("  ❌ Cancel failed: {}", e);
            }
        }
        println!();
    }

    Ok(())
}
