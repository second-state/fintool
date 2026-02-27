use anyhow::{bail, Result};
use colored::Colorize;
use serde_json::{json, Value};
use tabled::{settings::Style, Table, Tabled};

use crate::{binance, config, format};

#[derive(Tabled)]
struct PositionRow {
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Side")]
    side: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Entry")]
    entry: String,
    #[tabled(rename = "Mark")]
    mark: String,
    #[tabled(rename = "PnL")]
    pnl: String,
    #[tabled(rename = "Leverage")]
    leverage: String,
}

/// Resolve which exchange to use
fn resolve_exchange(exchange: &str) -> Result<String> {
    match exchange {
        "coinbase" => bail!("Positions not supported on Coinbase (spot-only exchange). Use --exchange hyperliquid or --exchange binance for perpetual futures positions."),
        "hyperliquid" | "binance" => Ok(exchange.to_string()),
        "auto" => {
            let has_hl = config::load_hl_config().is_ok();
            let has_binance = config::binance_credentials().is_some();

            // Priority for positions: Hyperliquid > Binance (Coinbase is spot-only)
            if has_hl {
                Ok("hyperliquid".to_string())
            } else if has_binance {
                Ok("binance".to_string())
            } else {
                bail!("No exchange configured for positions. Set up Hyperliquid wallet or Binance API keys in ~/.fintool/config.toml")
            }
        }
        _ => bail!(
            "Invalid exchange: {}. Use hyperliquid, binance, or auto",
            exchange
        ),
    }
}

pub async fn run(exchange: &str, json_output: bool) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured"))?;

        let client = reqwest::Client::new();
        let positions = binance::get_futures_positions(&client, &api_key, &api_secret).await?;

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "exchange": "binance",
                    "positions": positions,
                }))?
            );
            return Ok(());
        }

        let empty_vec = vec![];
        let active_positions: Vec<&Value> = positions
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter(|p| {
                let amt: f64 = p
                    .get("positionAmt")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                amt != 0.0
            })
            .collect();

        if active_positions.is_empty() {
            println!("\n  No open Binance futures positions.\n");
            return Ok(());
        }

        let rows: Vec<PositionRow> = active_positions
            .iter()
            .map(|p| {
                let symbol = p
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let amt: f64 = p
                    .get("positionAmt")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let entry: f64 = p
                    .get("entryPrice")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let mark: f64 = p
                    .get("markPrice")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let pnl: f64 = p
                    .get("unRealizedProfit")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let leverage: i64 = p
                    .get("leverage")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);

                let is_long = amt > 0.0;

                PositionRow {
                    symbol,
                    side: if is_long {
                        "LONG".green().to_string()
                    } else {
                        "SHORT".red().to_string()
                    },
                    size: format!("{:.8}", amt.abs()),
                    entry: format!("${:.2}", entry),
                    mark: format!("${:.2}", mark),
                    pnl: format::color_pnl(&format!("{:.2}", pnl)),
                    leverage: format!("{}x", leverage),
                }
            })
            .collect();

        println!("\n  📊 Binance Futures Positions\n");
        let table = Table::new(rows).with(Style::rounded()).to_string();
        for line in table.lines() {
            println!("  {}", line);
        }
        println!();

        return Ok(());
    }

    // Hyperliquid logic
    let cfg = config::load_hl_config()?;
    let client = reqwest::Client::new();
    let url = config::info_url();

    // Query main perp positions
    let resp: Value = client
        .post(&url)
        .json(&json!({"type": "clearinghouseState", "user": cfg.address}))
        .send()
        .await?
        .json()
        .await?;

    let mut positions = resp["assetPositions"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Also query HIP-3 dex positions (cash, xyz, km)
    for dex in &["cash", "xyz", "km"] {
        if let Ok(dex_resp) = client
            .post(&url)
            .json(&json!({"type": "clearinghouseState", "user": cfg.address, "dex": dex}))
            .send()
            .await
        {
            if let Ok(dex_val) = dex_resp.json::<Value>().await {
                if let Some(dex_positions) = dex_val["assetPositions"].as_array() {
                    positions.extend(dex_positions.iter().cloned());
                }
            }
        }
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&positions)?);
        return Ok(());
    }

    let active: Vec<&Value> = positions
        .iter()
        .filter(|p| {
            let sz = p["position"]["szi"].as_str().unwrap_or("0");
            sz != "0" && sz != "0.0"
        })
        .collect();

    if active.is_empty() {
        println!("\n  No open positions.\n");
        return Ok(());
    }

    let rows: Vec<PositionRow> = active
        .iter()
        .map(|p| {
            let pos = &p["position"];
            let szi = pos["szi"].as_str().unwrap_or("0");
            let is_long = !szi.starts_with('-');
            PositionRow {
                symbol: pos["coin"].as_str().unwrap_or("").to_string(),
                side: if is_long {
                    "LONG".green().to_string()
                } else {
                    "SHORT".red().to_string()
                },
                size: szi.to_string(),
                entry: format!("${}", pos["entryPx"].as_str().unwrap_or("-")),
                mark: format!("${}", pos["positionValue"].as_str().unwrap_or("-")),
                pnl: format::color_pnl(pos["unrealizedPnl"].as_str().unwrap_or("0")),
                leverage: format!(
                    "{}x",
                    pos["leverage"]["value"]
                        .as_str()
                        .or_else(|| pos["leverage"]["value"].as_f64().map(|_| ""))
                        .unwrap_or("-")
                ),
            }
        })
        .collect();

    println!("\n  📊 Open Positions\n");
    let table = Table::new(rows).with(Style::rounded()).to_string();
    for line in table.lines() {
        println!("  {}", line);
    }
    println!();

    Ok(())
}
