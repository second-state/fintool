use anyhow::{bail, Context, Result};
use colored::Colorize;
use hyperliquid_rust_sdk::{ExchangeDataStatus, ExchangeResponseStatus};
use serde_json::json;

use crate::{binance, coinbase, config, signing};

/// Parse the SDK ExchangeResponseStatus into (fill_status, json_value)
fn parse_sdk_order_result(result: &ExchangeResponseStatus) -> (String, serde_json::Value) {
    match result {
        ExchangeResponseStatus::Err(e) => (format!("error: {}", e), json!({"error": e})),
        ExchangeResponseStatus::Ok(resp) => {
            if let Some(statuses) = resp.data.as_ref().map(|d| &d.statuses) {
                if let Some(status) = statuses.first() {
                    match status {
                        ExchangeDataStatus::Filled(f) => (
                            "filled".to_string(),
                            json!({"filled": {"totalSz": f.total_sz, "avgPx": f.avg_px, "oid": f.oid}}),
                        ),
                        ExchangeDataStatus::Resting(r) => {
                            ("resting".to_string(), json!({"resting": {"oid": r.oid}}))
                        }
                        ExchangeDataStatus::Error(e) => {
                            (format!("error: {}", e), json!({"error": e}))
                        }
                        _ => (
                            format!("{:?}", status).to_lowercase(),
                            json!(format!("{:?}", status)),
                        ),
                    }
                } else {
                    ("unknown".to_string(), json!(format!("{:?}", resp)))
                }
            } else {
                ("unknown".to_string(), json!(format!("{:?}", resp)))
            }
        }
    }
}

/// Resolve which exchange to use
fn resolve_exchange(exchange: &str) -> Result<String> {
    match exchange {
        "hyperliquid" | "binance" | "coinbase" => Ok(exchange.to_string()),
        "auto" => {
            let has_hl = config::load_hl_config().is_ok();
            let has_coinbase = config::coinbase_credentials().is_some();
            let has_binance = config::binance_credentials().is_some();

            // Priority for spot/perp: Hyperliquid > Coinbase > Binance
            if has_hl {
                Ok("hyperliquid".to_string())
            } else if has_coinbase {
                Ok("coinbase".to_string())
            } else if has_binance {
                Ok("binance".to_string())
            } else {
                bail!("No exchange configured. Set up Hyperliquid wallet, Coinbase API keys, or Binance API keys in ~/.fintool/config.toml")
            }
        }
        _ => bail!(
            "Invalid exchange: {}. Use hyperliquid, binance, coinbase, or auto",
            exchange
        ),
    }
}

/// Spot limit buy — price is the maximum price you'll pay per unit
pub async fn buy(
    symbol: &str,
    amount_usdc: &str,
    max_price: &str,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "coinbase" {
        let (api_key, api_secret) = config::coinbase_credentials()
            .ok_or_else(|| anyhow::anyhow!("Coinbase API credentials not configured. Add coinbase_api_key and coinbase_api_secret to ~/.fintool/config.toml"))?;

        let price_f: f64 = max_price.parse().context("Invalid max price")?;
        let amount_f: f64 = amount_usdc.parse().context("Invalid amount")?;
        let size = amount_f / price_f;

        let client = reqwest::Client::new();
        return coinbase::spot_order(
            &client,
            &api_key,
            &api_secret,
            symbol,
            "BUY",
            size,
            price_f,
            json_output,
        )
        .await;
    }

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured. Add binance_api_key and binance_api_secret to ~/.fintool/config.toml"))?;

        let symbol = format!("{}USDT", symbol.to_uppercase());
        let price_f: f64 = max_price.parse().context("Invalid max price")?;
        let amount_f: f64 = amount_usdc.parse().context("Invalid amount")?;
        let size = amount_f / price_f;

        let client = reqwest::Client::new();
        return binance::spot_order(
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
    let price_f: f64 = max_price.parse().context("Invalid max price")?;
    let amount_f: f64 = amount_usdc.parse().context("Invalid amount")?;
    let size = amount_f / price_f;

    // Transfer USDC from perp to spot (required for spot buys in standard mode).
    // In unified mode, USDC is already shared — transfer will be rejected, which is fine.
    match signing::class_transfer(amount_f, false).await {
        Ok(()) => {
            if !json_output {
                eprintln!("  Transferred ${} USDC from perp → spot", amount_usdc);
            }
        }
        Err(e) => {
            let msg = format!("{:#}", e);
            if msg.contains("unified account is active") || msg.contains("Action disabled") {
                // Unified mode — USDC already available for spot, no transfer needed
            } else {
                return Err(e.context("Failed to transfer USDC from perp to spot"));
            }
        }
    }

    if !json_output {
        println!();
        println!("  📝 Placing spot limit BUY");
        println!("  Symbol:    {}", symbol.cyan());
        println!("  Size:      {:.6}", size);
        println!("  Max Price: ${}", max_price);
        println!("  Total:     ${}", amount_usdc);
        println!(
            "  Network:   {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = signing::place_spot_order(&symbol, true, price_f, size).await?;

    let (fill_status, result_json) = parse_sdk_order_result(&result);

    if fill_status.starts_with("error") {
        bail!("Spot order rejected: {}", fill_status);
    }

    let response = json!({
        "action": "spot_buy",
        "symbol": symbol,
        "size": format!("{:.6}", size),
        "maxPrice": max_price,
        "total_usdc": amount_usdc,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "fillStatus": fill_status,
        "result": result_json,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match &result {
            ExchangeResponseStatus::Ok(_) => {
                println!("  ✅ Spot buy order placed! ({})", fill_status);
            }
            ExchangeResponseStatus::Err(e) => {
                println!("  ❌ Order failed: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

/// Spot limit sell — price is the minimum price you'll accept per unit
pub async fn sell(
    symbol: &str,
    amount: &str,
    min_price: &str,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "coinbase" {
        let (api_key, api_secret) = config::coinbase_credentials()
            .ok_or_else(|| anyhow::anyhow!("Coinbase API credentials not configured. Add coinbase_api_key and coinbase_api_secret to ~/.fintool/config.toml"))?;

        let size: f64 = amount.parse().context("Invalid amount")?;
        let price_f: f64 = min_price.parse().context("Invalid min price")?;

        let client = reqwest::Client::new();
        return coinbase::spot_order(
            &client,
            &api_key,
            &api_secret,
            symbol,
            "SELL",
            size,
            price_f,
            json_output,
        )
        .await;
    }

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured. Add binance_api_key and binance_api_secret to ~/.fintool/config.toml"))?;

        let symbol = format!("{}USDT", symbol.to_uppercase());
        let size: f64 = amount.parse().context("Invalid amount")?;
        let price_f: f64 = min_price.parse().context("Invalid min price")?;

        let client = reqwest::Client::new();
        return binance::spot_order(
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
    let client = reqwest::Client::new();
    let symbol = symbol.to_uppercase();
    let size: f64 = amount.parse().context("Invalid amount")?;
    let price_f: f64 = min_price.parse().context("Invalid min price")?;

    if !json_output {
        println!();
        println!("  📝 Placing spot limit SELL");
        println!("  Symbol:    {}", symbol.cyan());
        println!("  Size:      {}", amount);
        println!("  Min Price: ${}", min_price);
        println!(
            "  Network:   {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = signing::place_spot_order(&symbol, false, price_f, size).await?;

    let (fill_status, result_json) = parse_sdk_order_result(&result);

    if fill_status.starts_with("error") {
        bail!("Spot order rejected: {}", fill_status);
    }

    let response = json!({
        "action": "spot_sell",
        "symbol": symbol,
        "size": amount,
        "minPrice": min_price,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "fillStatus": fill_status,
        "result": result_json,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match &result {
            ExchangeResponseStatus::Ok(_) => {
                println!("  ✅ Spot sell order placed! ({})", fill_status);
            }
            ExchangeResponseStatus::Err(e) => {
                println!("  ❌ Order failed: {}", e);
            }
        }
        println!();
    }

    // After a filled sell, sweep USDC from spot back to perp (standard mode only).
    // In unified mode, USDC is already shared — no sweep needed.
    if fill_status == "filled" {
        let url = config::info_url();
        let spot_state: serde_json::Value = client
            .post(&url)
            .json(&json!({"type": "spotClearinghouseState", "user": cfg.address}))
            .send()
            .await?
            .json()
            .await?;

        let usdc_total: f64 = spot_state["balances"]
            .as_array()
            .and_then(|balances: &Vec<serde_json::Value>| {
                balances.iter().find_map(|b| {
                    if b["coin"].as_str() == Some("USDC") {
                        b["total"].as_str().and_then(|s| s.parse::<f64>().ok())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0.0);

        if usdc_total > 0.01 {
            match signing::class_transfer(usdc_total, true).await {
                Ok(()) => {
                    if !json_output {
                        eprintln!("  Transferred ${:.2} USDC from spot → perp", usdc_total);
                    }
                }
                Err(e) => {
                    let msg = format!("{:#}", e);
                    if !msg.contains("unified account is active")
                        && !msg.contains("Action disabled")
                    {
                        eprintln!("  Warning: failed to transfer USDC back to perp: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
