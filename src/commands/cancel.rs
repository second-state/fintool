use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::{binance, config, signing};

/// Resolve which exchange to use
fn resolve_exchange(exchange: &str) -> Result<String> {
    match exchange {
        "hyperliquid" | "binance" => Ok(exchange.to_string()),
        "auto" => {
            let has_hl = config::load_hl_config().is_ok();
            let has_binance = config::binance_credentials().is_some();

            if has_hl && !has_binance {
                Ok("hyperliquid".to_string())
            } else if has_binance && !has_hl {
                Ok("binance".to_string())
            } else if has_hl && has_binance {
                // Default to Hyperliquid
                Ok("hyperliquid".to_string())
            } else {
                bail!("No exchange configured. Set up Hyperliquid wallet or Binance API keys in ~/.fintool/config.toml")
            }
        }
        _ => bail!(
            "Invalid exchange: {}. Use hyperliquid, binance, or auto",
            exchange
        ),
    }
}

pub async fn run(order_id: &str, exchange: &str, json_output: bool) -> Result<()> {
    // Check if order ID is prefixed with exchange identifier
    if order_id.starts_with("binance_spot:") || order_id.starts_with("binance_futures:") {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured"))?;

        let parts: Vec<&str> = order_id.split(':').collect();
        if parts.len() < 2 {
            bail!("Invalid Binance order ID format");
        }

        let market = parts[0]; // binance_spot or binance_futures

        // For Binance, we need the symbol. This is a limitation - ideally we'd store it
        // For now, require format: binance_spot:SYMBOL:ORDERID or binance_futures:SYMBOL:ORDERID
        if parts.len() < 3 {
            bail!("Binance order cancellation requires format: binance_spot:SYMBOL:ORDERID or binance_futures:SYMBOL:ORDERID");
        }

        let symbol = parts[1];
        let oid: u64 = parts[2].parse().context("Invalid Binance order ID")?;

        let client = reqwest::Client::new();

        let result = if market == "binance_spot" {
            binance::cancel_spot_order(&client, &api_key, &api_secret, symbol, oid).await?
        } else {
            binance::cancel_futures_order(&client, &api_key, &api_secret, symbol, oid).await?
        };

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "exchange": "binance",
                    "market": market,
                    "symbol": symbol,
                    "orderId": oid,
                    "result": result,
                }))?
            );
        } else {
            println!(
                "\n  ✅ Binance {} order cancelled!",
                if market == "binance_spot" {
                    "spot"
                } else {
                    "futures"
                }
            );
            println!("  Symbol:   {}", symbol.cyan());
            println!("  Order ID: {}\n", oid);
        }

        return Ok(());
    }

    let exchange = resolve_exchange(exchange)?;

    if exchange == "binance" {
        bail!("For Binance orders, use order ID format from `fintool orders --exchange binance`: binance_spot:SYMBOL:ORDERID or binance_futures:SYMBOL:ORDERID");
    }

    // Hyperliquid logic
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
