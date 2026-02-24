use anyhow::{Result, Context};
use colored::Colorize;
use serde_json::json;

use crate::config;
use crate::signing;

/// Spot limit buy — price is the maximum price you'll pay per unit
pub async fn buy(symbol: &str, amount_usdc: &str, max_price: &str, json_output: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let symbol = symbol.to_uppercase();
    let price_f: f64 = max_price.parse().context("Invalid max price")?;
    let amount_f: f64 = amount_usdc.parse().context("Invalid amount")?;
    let size = amount_f / price_f;

    if !json_output {
        println!();
        println!("  {} Placing spot limit BUY", "📝");
        println!("  Symbol:    {}", symbol.cyan());
        println!("  Size:      {:.6}", size);
        println!("  Max Price: ${}", max_price);
        println!("  Total:     ${}", amount_usdc);
        println!("  Network:   {}", if cfg.testnet { "Testnet" } else { "Mainnet" });
        println!();
    }

    let result = signing::place_spot_order(&symbol, true, price_f, size).await?;

    let response = json!({
        "action": "spot_buy",
        "symbol": symbol,
        "size": format!("{:.6}", size),
        "maxPrice": max_price,
        "total_usdc": amount_usdc,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match result {
            hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(data) => {
                println!("  {} Spot buy order placed!", "✅".green());
                println!("  Response: {:?}", data);
            }
            hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
                println!("  {} Order failed: {}", "❌".red(), e);
            }
        }
        println!();
    }

    Ok(())
}

/// Spot limit sell — price is the minimum price you'll accept per unit
pub async fn sell(symbol: &str, amount: &str, min_price: &str, json_output: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let symbol = symbol.to_uppercase();
    let size: f64 = amount.parse().context("Invalid amount")?;
    let price_f: f64 = min_price.parse().context("Invalid min price")?;

    if !json_output {
        println!();
        println!("  {} Placing spot limit SELL", "📝");
        println!("  Symbol:    {}", symbol.cyan());
        println!("  Size:      {}", amount);
        println!("  Min Price: ${}", min_price);
        println!("  Network:   {}", if cfg.testnet { "Testnet" } else { "Mainnet" });
        println!();
    }

    let result = signing::place_spot_order(&symbol, false, price_f, size).await?;

    let response = json!({
        "action": "spot_sell",
        "symbol": symbol,
        "size": amount,
        "minPrice": min_price,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match result {
            hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(data) => {
                println!("  {} Spot sell order placed!", "✅".green());
                println!("  Response: {:?}", data);
            }
            hyperliquid_rust_sdk::ExchangeResponseStatus::Err(e) => {
                println!("  {} Order failed: {}", "❌".red(), e);
            }
        }
        println!();
    }

    Ok(())
}
