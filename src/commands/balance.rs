use anyhow::{bail, Result};
use colored::Colorize;
use serde_json::{json, Value};
use tabled::{settings::Style, Table, Tabled};

use crate::{binance, coinbase, config};

#[derive(Tabled)]
struct BalanceRow {
    #[tabled(rename = "Asset")]
    asset: String,
    #[tabled(rename = "Total")]
    total: String,
    #[tabled(rename = "Available")]
    available: String,
    #[tabled(rename = "In Positions")]
    in_positions: String,
}

/// Resolve which exchange to use
fn resolve_exchange(exchange: &str) -> Result<String> {
    match exchange {
        "hyperliquid" | "binance" | "coinbase" => Ok(exchange.to_string()),
        "auto" => {
            let has_hl = config::load_hl_config().is_ok();
            let has_coinbase = config::coinbase_credentials().is_some();
            let has_binance = config::binance_credentials().is_some();

            // Priority: Hyperliquid > Coinbase > Binance
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

pub async fn run(exchange: &str, json_output: bool) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "coinbase" {
        let (api_key, api_secret) = config::coinbase_credentials()
            .ok_or_else(|| anyhow::anyhow!("Coinbase API credentials not configured"))?;

        let client = reqwest::Client::new();
        return coinbase::get_accounts(&client, &api_key, &api_secret, json_output).await;
    }

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured"))?;

        let client = reqwest::Client::new();

        // Get spot balances
        let spot = binance::get_spot_balances(&client, &api_key, &api_secret).await?;

        // Get futures balances
        let futures = binance::get_futures_balances(&client, &api_key, &api_secret).await?;

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "exchange": "binance",
                    "spot": spot,
                    "futures": futures,
                }))?
            );
            return Ok(());
        }

        println!();
        println!("  💰 Binance Account Balance");
        println!();
        println!("  📊 Spot Balances:");

        if let Some(balances) = spot.get("balances").and_then(|v| v.as_array()) {
            for balance in balances {
                let asset = balance.get("asset").and_then(|v| v.as_str()).unwrap_or("");
                let free: f64 = balance
                    .get("free")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let locked: f64 = balance
                    .get("locked")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);

                if free > 0.0 || locked > 0.0 {
                    println!(
                        "    {}: {} (available: {}, locked: {})",
                        asset.cyan(),
                        free + locked,
                        free,
                        locked
                    );
                }
            }
        }

        println!();
        println!("  📈 Futures Balances:");

        if let Some(balances) = futures.as_array() {
            for balance in balances {
                let asset = balance.get("asset").and_then(|v| v.as_str()).unwrap_or("");
                let balance_val: f64 = balance
                    .get("balance")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let available: f64 = balance
                    .get("availableBalance")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);

                if balance_val > 0.0 || available > 0.0 {
                    println!(
                        "    {}: {} (available: {})",
                        asset.cyan(),
                        balance_val,
                        available
                    );
                }
            }
        }

        println!();
        return Ok(());
    }

    // Hyperliquid logic
    let cfg = config::load_hl_config()?;
    let client = reqwest::Client::new();
    let url = config::info_url();

    let resp: Value = client
        .post(&url)
        .json(&json!({"type": "clearinghouseState", "user": cfg.address}))
        .send()
        .await?
        .json()
        .await?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    let margin = &resp["marginSummary"];
    let account_value = margin["accountValue"].as_str().unwrap_or("0");
    let total_margin = margin["totalMarginUsed"].as_str().unwrap_or("0");
    let available = margin["totalNtlPos"].as_str().unwrap_or("0");

    println!();
    println!("  💰 Account Balance");
    println!();

    let rows = vec![BalanceRow {
        asset: "USDC".to_string(),
        total: format!("${}", account_value),
        available: format!("${}", available),
        in_positions: format!("${}", total_margin),
    }];

    let table = Table::new(rows).with(Style::rounded()).to_string();
    for line in table.lines() {
        println!("  {}", line);
    }

    // Cross margin details
    if let Some(cross) = resp.get("crossMarginSummary") {
        println!();
        println!(
            "  Account Value:   ${}",
            cross["accountValue"].as_str().unwrap_or("-").green()
        );
        println!(
            "  Total Margin:    ${}",
            cross["totalMarginUsed"].as_str().unwrap_or("-")
        );
        println!(
            "  Notional:        ${}",
            cross["totalNtlPos"].as_str().unwrap_or("-")
        );
    }
    println!();

    Ok(())
}
