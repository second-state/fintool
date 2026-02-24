use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::config;
use crate::signing;

pub async fn buy(symbol: &str, amount_usdc: &str, price: &str, json_output: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let symbol = symbol.to_uppercase();
    let price_f: f64 = price.parse().context("Invalid price")?;
    let amount_f: f64 = amount_usdc.parse().context("Invalid amount")?;
    let size = amount_f / price_f;

    if !json_output {
        println!();
        println!("  📝 Placing perp limit BUY (long)");
        println!("  Symbol:   {}", symbol.cyan());
        println!("  Size:     {:.6}", size);
        println!("  Price:    ${}", price);
        println!("  Total:    ${}", amount_usdc);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    // Perp orders use the symbol directly (e.g. "BTC")
    let result = signing::place_perp_order(&symbol, true, price_f, size).await?;

    let response = json!({
        "action": "perp_buy",
        "symbol": symbol,
        "size": format!("{:.6}", size),
        "price": price,
        "total_usdc": amount_usdc,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match result {
            hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(data) => {
                println!("  ✅ Perp order placed!");
                println!("  Response: {:?}", data);
            }
            hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
                println!("  ❌ Order failed: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

pub async fn sell(symbol: &str, amount: &str, price: &str, json_output: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let symbol = symbol.to_uppercase();
    let size: f64 = amount.parse().context("Invalid amount")?;
    let price_f: f64 = price.parse().context("Invalid price")?;

    if !json_output {
        println!();
        println!("  📝 Placing perp limit SELL (short)");
        println!("  Symbol:   {}", symbol.cyan());
        println!("  Size:     {}", amount);
        println!("  Price:    ${}", price);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = signing::place_perp_order(&symbol, false, price_f, size).await?;

    let response = json!({
        "action": "perp_sell",
        "symbol": symbol,
        "size": amount,
        "price": price,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match result {
            hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(data) => {
                println!("  ✅ Perp order placed!");
                println!("  Response: {:?}", data);
            }
            hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
                println!("  ❌ Order failed: {}", e);
            }
        }
        println!();
    }

    Ok(())
}
