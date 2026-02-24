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
                // Default to Hyperliquid for spot/perp when both configured
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

pub async fn buy(
    symbol: &str,
    amount_usdc: &str,
    price: &str,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured. Add binance_api_key and binance_api_secret to ~/.fintool/config.toml"))?;

        let symbol = format!("{}USDT", symbol.to_uppercase());
        let price_f: f64 = price.parse().context("Invalid price")?;
        let amount_f: f64 = amount_usdc.parse().context("Invalid amount")?;
        let size = amount_f / price_f;

        let client = reqwest::Client::new();
        return binance::futures_order(
            &client,
            &api_key,
            &api_secret,
            &symbol,
            "BUY",
            size,
            price_f,
            json_output,
        )
        .await;
    }

    // Hyperliquid logic
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

pub async fn sell(
    symbol: &str,
    amount: &str,
    price: &str,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured. Add binance_api_key and binance_api_secret to ~/.fintool/config.toml"))?;

        let symbol = format!("{}USDT", symbol.to_uppercase());
        let size: f64 = amount.parse().context("Invalid amount")?;
        let price_f: f64 = price.parse().context("Invalid price")?;

        let client = reqwest::Client::new();
        return binance::futures_order(
            &client,
            &api_key,
            &api_secret,
            &symbol,
            "SELL",
            size,
            price_f,
            json_output,
        )
        .await;
    }

    // Hyperliquid logic
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
