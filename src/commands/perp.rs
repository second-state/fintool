use anyhow::{bail, Context, Result};
use colored::Colorize;
use hyperliquid_rust_sdk::{ExchangeDataStatus, ExchangeResponseStatus};
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
    close: bool,
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
        return hip3_perp_buy(
            &dex,
            &asset_name,
            &symbol,
            price_f,
            size,
            amount_usdc,
            &cfg,
            json_output,
        )
        .await;
    }

    let mode = if close {
        "CLOSE (reduce-only)"
    } else {
        "BUY (long)"
    };
    if !json_output {
        println!();
        println!("  📝 Placing perp limit {}", mode);
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
    let result = signing::place_perp_order(&symbol, true, price_f, size, close).await?;

    let (fill_status, result_json) = parse_sdk_order_result(&result);

    if fill_status.starts_with("error") {
        bail!("Perp order rejected: {}", fill_status);
    }

    let response = json!({
        "action": "perp_buy",
        "symbol": symbol,
        "size": format!("{:.6}", size),
        "price": price,
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
                println!("  ✅ Perp order placed! ({})", fill_status);
            }
            ExchangeResponseStatus::Err(e) => {
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
    close: bool,
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

    let mode = if close {
        "CLOSE (reduce-only)"
    } else {
        "SELL (short)"
    };
    if !json_output {
        println!();
        println!("  📝 Placing perp limit {}", mode);
        println!("  Symbol:   {}", symbol.cyan());
        println!("  Size:     {}", amount);
        println!("  Price:    ${}", price);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    let result = signing::place_perp_order(&symbol, false, price_f, size, close).await?;

    let (fill_status, result_json) = parse_sdk_order_result(&result);

    if fill_status.starts_with("error") {
        bail!("Perp order rejected: {}", fill_status);
    }

    let response = json!({
        "action": "perp_sell",
        "symbol": symbol,
        "size": amount,
        "price": price,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "fillStatus": fill_status,
        "result": result_json,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match &result {
            ExchangeResponseStatus::Ok(_) => {
                println!("  ✅ Perp order placed! ({})", fill_status);
            }
            ExchangeResponseStatus::Err(e) => {
                println!("  ❌ Order failed: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

pub async fn set_leverage(
    symbol: &str,
    leverage: u32,
    is_cross: bool,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;
    let symbol = symbol.to_uppercase();
    let margin_type = if is_cross { "cross" } else { "isolated" };

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured. Add binance_api_key and binance_api_secret to ~/.fintool/config.toml"))?;

        let binance_symbol = format!("{}USDT", symbol);
        let client = reqwest::Client::new();
        return binance::set_leverage(
            &client,
            &api_key,
            &api_secret,
            &binance_symbol,
            leverage,
            json_output,
        )
        .await;
    }

    // Hyperliquid
    let cfg = config::load_hl_config()?;

    if !json_output {
        println!();
        println!("  📝 Setting leverage");
        println!("  Symbol:   {}", symbol.cyan());
        println!("  Leverage: {}x", leverage);
        println!("  Margin:   {}", margin_type);
        println!(
            "  Network:  {}",
            if cfg.testnet { "Testnet" } else { "Mainnet" }
        );
        println!();
    }

    // Check if this is a HIP-3 asset (commodities, stocks on dexes)
    if let Some((dex, asset_name)) = resolve_hip3_asset(&symbol) {
        let result = hip3::set_leverage(&dex, &asset_name, leverage, is_cross).await?;

        let response = json!({
            "action": "set_leverage",
            "exchange": "hyperliquid",
            "symbol": symbol,
            "hip3Asset": asset_name,
            "dex": dex,
            "leverage": leverage,
            "marginType": margin_type,
            "network": if cfg.testnet { "testnet" } else { "mainnet" },
            "result": result,
        });

        if json_output {
            println!("{}", serde_json::to_string_pretty(&response)?);
        } else {
            println!(
                "  ✅ Leverage set to {}x ({}) for {}",
                leverage, margin_type, symbol
            );
            println!();
        }

        return Ok(());
    }

    // Main perp asset (BTC, ETH, etc.)
    let result = signing::set_leverage(&symbol, leverage, is_cross).await?;

    let response = json!({
        "action": "set_leverage",
        "exchange": "hyperliquid",
        "symbol": symbol,
        "leverage": leverage,
        "marginType": margin_type,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "result": format!("{:?}", result),
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match &result {
            ExchangeResponseStatus::Ok(_) => {
                println!(
                    "  ✅ Leverage set to {}x ({}) for {}",
                    leverage, margin_type, symbol
                );
            }
            ExchangeResponseStatus::Err(e) => {
                bail!("Failed to set leverage: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

pub async fn set_mode(mode: &str, json_output: bool) -> Result<()> {
    let api_mode = match mode.to_lowercase().as_str() {
        "unified" | "unifiedaccount" => "unifiedAccount",
        "standard" => "standard",
        "disabled" => "disabled",
        _ => bail!(
            "Invalid mode: {}. Use 'unified', 'standard', or 'disabled'",
            mode
        ),
    };

    config::load_hl_config()
        .context("Hyperliquid wallet not configured. Set-mode requires Hyperliquid.")?;

    signing::set_abstraction(api_mode).await?;

    let response = json!({
        "action": "set_mode",
        "mode": api_mode,
        "status": "ok",
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("  ✅ Account mode set to: {}", api_mode);
    }

    Ok(())
}

// ── Order result helpers ─────────────────────────────────────────────

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

/// Parse the Hyperliquid HIP-3 order response to determine fill status.
/// Returns "filled", "resting", "error: ...", or "unknown".
fn parse_order_status(result: &serde_json::Value) -> String {
    let status = result
        .get("response")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("statuses"))
        .and_then(|s| s.as_array())
        .and_then(|a| a.first());

    match status {
        Some(s) if s.get("filled").is_some() => "filled".to_string(),
        Some(s) if s.get("resting").is_some() => "resting".to_string(),
        Some(s) if s.get("error").is_some() => {
            format!("error: {}", s["error"].as_str().unwrap_or("unknown"))
        }
        _ => "unknown".to_string(),
    }
}

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

    let fill_status = parse_order_status(&result);

    let response = json!({
        "action": "hip3_perp_buy",
        "symbol": original_symbol,
        "hip3Asset": asset_name,
        "dex": dex,
        "size": format!("{:.6}", size),
        "price": format!("{:.2}", price),
        "total_usdc": amount_usdc,
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "fillStatus": fill_status,
        "result": result,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
            if status == "ok" {
                println!("  ✅ HIP-3 perp order placed! ({})", fill_status);
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

    let fill_status = parse_order_status(&result);

    let response = json!({
        "action": "hip3_perp_sell",
        "symbol": original_symbol,
        "hip3Asset": asset_name,
        "dex": dex,
        "size": format!("{:.6}", size),
        "price": format!("{:.2}", price),
        "network": if cfg.testnet { "testnet" } else { "mainnet" },
        "fillStatus": fill_status,
        "result": result,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
            if status == "ok" {
                println!("  ✅ HIP-3 perp order placed! ({})", fill_status);
            } else {
                println!("  ❌ Order failed: {}", status);
            }
        }
        println!("  Response: {}", serde_json::to_string_pretty(&result)?);
        println!();
    }

    Ok(())
}
