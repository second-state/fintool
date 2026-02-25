use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::commands::quote::commodity_to_spot_token;
use crate::{binance, config, signing};

/// Resolve which exchange to use
fn resolve_exchange(exchange: &str) -> Result<String> {
    match exchange {
        "coinbase" => bail!("Perpetual futures not supported on Coinbase. Use --exchange hyperliquid or --exchange binance"),
        "hyperliquid" | "binance" => Ok(exchange.to_string()),
        "auto" => {
            let has_hl = config::load_hl_config().is_ok();
            let has_binance = config::binance_credentials().is_some();

            // Priority for perp: Hyperliquid > Binance (Coinbase doesn't support perps)
            if has_hl {
                Ok("hyperliquid".to_string())
            } else if has_binance {
                Ok("binance".to_string())
            } else {
                bail!("No exchange configured for perpetual futures. Set up Hyperliquid wallet or Binance API keys in ~/.fintool/config.toml")
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

    // Check if this is a commodity → route to spot order
    if let Some(spot_token) = commodity_to_spot_token(&symbol) {
        return commodity_spot_buy(spot_token, &symbol, price_f, size, &cfg, json_output).await;
    }

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

    // Check if this is a commodity → route to spot order
    if let Some(spot_token) = commodity_to_spot_token(&symbol) {
        return commodity_spot_sell(spot_token, &symbol, price_f, size, &cfg, json_output).await;
    }

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

// ── Commodity spot order helpers ─────────────────────────────────────

/// Route a commodity "perp buy" to a spot buy on HL
async fn commodity_spot_buy(
    spot_token: &str,
    original_symbol: &str,
    price: f64,
    size: f64,
    cfg: &config::HlConfig,
    json_output: bool,
) -> Result<()> {
    if !json_output {
        println!();
        println!(
            "  📝 {} trades as {}/USDC spot on Hyperliquid",
            original_symbol,
            spot_token
        );
        println!("  Placing spot limit BUY");
        println!("  Token:    {}", spot_token.cyan());
        println!("  Size:     {:.6}", size);
        println!("  Price:    ${:.2}", price);
        println!("  Total:    ${:.2}", price * size);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = signing::place_spot_order(spot_token, true, price, size).await?;

    let response = json!({
        "action": "commodity_spot_buy",
        "symbol": original_symbol,
        "spotToken": spot_token,
        "size": format!("{:.6}", size),
        "price": format!("{:.2}", price),
        "total_usdc": format!("{:.2}", price * size),
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "note": format!("{} trades as {}/USDC spot pair (no perp available)", original_symbol, spot_token),
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match result {
            hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(data) => {
                println!("  ✅ Spot order placed!");
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

/// Route a commodity "perp sell" to a spot sell on HL
async fn commodity_spot_sell(
    spot_token: &str,
    original_symbol: &str,
    price: f64,
    size: f64,
    cfg: &config::HlConfig,
    json_output: bool,
) -> Result<()> {
    if !json_output {
        println!();
        println!(
            "  📝 {} trades as {}/USDC spot on Hyperliquid",
            original_symbol,
            spot_token
        );
        println!("  Placing spot limit SELL");
        println!("  Token:    {}", spot_token.cyan());
        println!("  Size:     {:.6}", size);
        println!("  Price:    ${:.2}", price);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = signing::place_spot_order(spot_token, false, price, size).await?;

    let response = json!({
        "action": "commodity_spot_sell",
        "symbol": original_symbol,
        "spotToken": spot_token,
        "size": format!("{:.6}", size),
        "price": format!("{:.2}", price),
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "note": format!("{} trades as {}/USDC spot pair (no perp available)", original_symbol, spot_token),
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match result {
            hyperliquid_rust_sdk::ExchangeResponseStatus::Ok(data) => {
                println!("  ✅ Spot order placed!");
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
