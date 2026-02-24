use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::{binance, config};

/// Resolve which exchange to use for options
fn resolve_exchange(exchange: &str) -> Result<String> {
    match exchange {
        "binance" => Ok("binance".to_string()),
        "coinbase" => bail!("Options not supported on Coinbase. Use --exchange binance or configure Binance API keys."),
        "hyperliquid" => bail!("Options not supported on Hyperliquid. Use --exchange binance or configure Binance API keys."),
        "auto" => {
            let has_binance = config::binance_credentials().is_some();

            if has_binance {
                Ok("binance".to_string())
            } else {
                bail!("Options trading requires Binance. Configure binance_api_key and binance_api_secret in ~/.fintool/config.toml")
            }
        }
        _ => bail!("Invalid exchange: {}. Options are only supported on Binance", exchange),
    }
}

pub async fn buy(
    symbol: &str,
    option_type: &str,
    strike: &str,
    expiry: &str,
    size: &str,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured. Add binance_api_key and binance_api_secret to ~/.fintool/config.toml"))?;

        let strike_f: f64 = strike.parse().context("Invalid strike price")?;
        let qty: f64 = size.parse().context("Invalid size")?;

        let client = reqwest::Client::new();
        return binance::options_order(
            &client,
            &api_key,
            &api_secret,
            symbol,
            "BUY",
            option_type,
            strike_f,
            expiry,
            qty,
            json_output,
        )
        .await;
    }

    // Fallback (shouldn't reach here due to resolve_exchange)
    if json_output {
        println!(
            "{}",
            serde_json::json!({
                "status": "not_implemented",
                "note": "Native options support coming with Hyperliquid HIP-4",
                "params": { "symbol": symbol, "type": option_type, "strike": strike, "expiry": expiry, "size": size }
            })
        );
    } else {
        println!();
        println!("  📋 Options Buy (Stub)");
        println!("  Symbol: {}", symbol.cyan());
        println!("  Type:   {}", option_type);
        println!("  Strike: ${}", strike);
        println!("  Expiry: {}", expiry);
        println!("  Size:   {}", size);
        println!();
        println!(
            "  {} Native options support coming with Hyperliquid HIP-4.",
            "ℹ️".blue()
        );
        println!("  Currently, options-like exposure can be achieved via perps with stop-losses.");
        println!();
    }
    Ok(())
}

pub async fn sell(
    symbol: &str,
    option_type: &str,
    strike: &str,
    expiry: &str,
    size: &str,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured. Add binance_api_key and binance_api_secret to ~/.fintool/config.toml"))?;

        let strike_f: f64 = strike.parse().context("Invalid strike price")?;
        let qty: f64 = size.parse().context("Invalid size")?;

        let client = reqwest::Client::new();
        return binance::options_order(
            &client,
            &api_key,
            &api_secret,
            symbol,
            "SELL",
            option_type,
            strike_f,
            expiry,
            qty,
            json_output,
        )
        .await;
    }

    // Fallback (shouldn't reach here due to resolve_exchange)
    if json_output {
        println!(
            "{}",
            serde_json::json!({
                "status": "not_implemented",
                "note": "Native options support coming with Hyperliquid HIP-4",
                "params": { "symbol": symbol, "type": option_type, "strike": strike, "expiry": expiry, "size": size }
            })
        );
    } else {
        println!();
        println!("  📋 Options Sell (Stub)");
        println!("  Symbol: {}", symbol.cyan());
        println!("  Type:   {}", option_type);
        println!("  Strike: ${}", strike);
        println!("  Expiry: {}", expiry);
        println!("  Size:   {}", size);
        println!();
        println!(
            "  {} Native options support coming with Hyperliquid HIP-4.",
            "ℹ️".blue()
        );
        println!();
    }
    Ok(())
}
