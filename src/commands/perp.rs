use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::commands::quote::resolve_hip3_asset;
use crate::{binance, config, hip3, signing};

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

    // Check if this is a HIP-3 asset (commodities, stocks)
    if let Some((dex, asset_name)) = resolve_hip3_asset(&symbol) {
        return hip3_perp_buy(&dex, &asset_name, &symbol, price_f, size, amount_usdc, &cfg, json_output).await;
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

    // Check if this is a HIP-3 asset (commodities, stocks)
    if let Some((dex, asset_name)) = resolve_hip3_asset(&symbol) {
        return hip3_perp_sell(&dex, &asset_name, &symbol, price_f, size, &cfg, json_output).await;
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

// ── HIP-3 perp order helpers ─────────────────────────────────────────

/// Place a HIP-3 perp buy order (e.g. cash:SILVER)
#[allow(clippy::too_many_arguments)]
async fn hip3_perp_buy(
    dex: &str,
    asset_name: &str,
    original_symbol: &str,
    price: f64,
    size: f64,
    amount_usdc: &str,
    cfg: &config::HlConfig,
    json_output: bool,
) -> Result<()> {
    if !json_output {
        println!();
        println!(
            "  📝 Placing HIP-3 perp limit BUY ({} on {} dex)",
            asset_name.cyan(),
            dex
        );
        println!("  Symbol:   {}", original_symbol.cyan());
        println!("  Asset:    {}", asset_name);
        println!("  Size:     {:.6}", size);
        println!("  Price:    ${:.2}", price);
        println!("  Total:    ${}", amount_usdc);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = hip3::place_order(dex, asset_name, true, price, size).await?;

    let response = json!({
        "action": "hip3_perp_buy",
        "symbol": original_symbol,
        "hip3Asset": asset_name,
        "dex": dex,
        "size": format!("{:.6}", size),
        "price": format!("{:.2}", price),
        "total_usdc": amount_usdc,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": result,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
            if status == "ok" {
                println!("  ✅ HIP-3 perp order placed!");
            } else {
                println!("  ❌ Order failed: {}", status);
            }
        }
        println!("  Response: {}", serde_json::to_string_pretty(&result)?);
        println!();
    }

    Ok(())
}

/// Place a HIP-3 perp sell order
async fn hip3_perp_sell(
    dex: &str,
    asset_name: &str,
    original_symbol: &str,
    price: f64,
    size: f64,
    cfg: &config::HlConfig,
    json_output: bool,
) -> Result<()> {
    if !json_output {
        println!();
        println!(
            "  📝 Placing HIP-3 perp limit SELL ({} on {} dex)",
            asset_name.cyan(),
            dex
        );
        println!("  Symbol:   {}", original_symbol.cyan());
        println!("  Asset:    {}", asset_name);
        println!("  Size:     {:.6}", size);
        println!("  Price:    ${:.2}", price);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = hip3::place_order(dex, asset_name, false, price, size).await?;

    let response = json!({
        "action": "hip3_perp_sell",
        "symbol": original_symbol,
        "hip3Asset": asset_name,
        "dex": dex,
        "size": format!("{:.6}", size),
        "price": format!("{:.2}", price),
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": result,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
            if status == "ok" {
                println!("  ✅ HIP-3 perp order placed!");
            } else {
                println!("  ❌ Order failed: {}", status);
            }
        }
        println!("  Response: {}", serde_json::to_string_pretty(&result)?);
        println!();
    }

    Ok(())
}
