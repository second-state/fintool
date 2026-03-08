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
        "hyperliquid" | "binance" | "coinbase" | "polymarket" => Ok(exchange.to_string()),
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
            "Invalid exchange: {}. Use hyperliquid, binance, coinbase, polymarket, or auto",
            exchange
        ),
    }
}

pub async fn run(exchange: &str, json_output: bool) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "polymarket" {
        return balance_polymarket(json_output).await;
    }

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

    // Fetch perp clearinghouse state
    let perp_resp: Value = client
        .post(&url)
        .json(&json!({"type": "clearinghouseState", "user": cfg.address}))
        .send()
        .await?
        .json()
        .await?;

    // Fetch spot clearinghouse state
    let spot_resp: Value = client
        .post(&url)
        .json(&json!({"type": "spotClearinghouseState", "user": cfg.address}))
        .send()
        .await?
        .json()
        .await?;

    if json_output {
        let combined = json!({
            "perp": perp_resp,
            "spot": spot_resp,
        });
        println!("{}", serde_json::to_string_pretty(&combined)?);
        return Ok(());
    }

    // ── Perp balance ──
    let margin = &perp_resp["marginSummary"];
    let account_value = margin["accountValue"].as_str().unwrap_or("0");
    let total_margin = margin["totalMarginUsed"].as_str().unwrap_or("0");
    let notional = margin["totalNtlPos"].as_str().unwrap_or("0");
    let withdrawable = perp_resp["withdrawable"].as_str().unwrap_or("0");

    println!();
    println!("  💰 Perp Account");
    println!();

    let rows = vec![BalanceRow {
        asset: "USDC".to_string(),
        total: format!("${}", account_value),
        available: format!("${}", withdrawable),
        in_positions: format!("${}", total_margin),
    }];

    let table = Table::new(&rows).with(Style::rounded()).to_string();
    for line in table.lines() {
        println!("  {}", line);
    }

    println!();
    println!("  Account Value:   ${}", account_value.green());
    println!("  Margin Used:     ${}", total_margin);
    println!("  Notional:        ${}", notional);
    println!("  Withdrawable:    ${}", withdrawable);

    // ── Spot balances ──
    println!();
    println!("  💰 Spot Account");
    println!();

    if let Some(balances) = spot_resp.get("balances").and_then(|b| b.as_array()) {
        let mut spot_rows: Vec<BalanceRow> = Vec::new();
        for bal in balances {
            let coin = bal["coin"].as_str().unwrap_or("?");
            let total = bal["total"].as_str().unwrap_or("0");
            let hold = bal["hold"].as_str().unwrap_or("0");
            let total_f: f64 = total.parse().unwrap_or(0.0);
            let hold_f: f64 = hold.parse().unwrap_or(0.0);
            let avail = total_f - hold_f;

            if total_f > 0.0 {
                spot_rows.push(BalanceRow {
                    asset: coin.to_string(),
                    total: total.to_string(),
                    available: format!("{:.6}", avail),
                    in_positions: hold.to_string(),
                });
            }
        }

        if spot_rows.is_empty() {
            println!("  (no spot balances)");
        } else {
            let table = Table::new(&spot_rows).with(Style::rounded()).to_string();
            for line in table.lines() {
                println!("  {}", line);
            }
        }
    } else {
        println!("  (no spot balances)");
    }

    println!();

    Ok(())
}

// ── Polymarket balance ───────────────────────────────────────────────

async fn balance_polymarket(json_output: bool) -> Result<()> {
    use polymarket_client_sdk::clob::types::request::BalanceAllowanceRequest;
    use polymarket_client_sdk::clob::types::AssetType;

    let clob = crate::polymarket::create_clob_client().await?;
    let req = BalanceAllowanceRequest::builder()
        .asset_type(AssetType::Collateral)
        .build();
    let resp = clob.balance_allowance(req).await?;
    let balance = resp.balance;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "polymarket",
                "asset": "USDC",
                "balance": balance.to_string(),
            }))?
        );
    } else {
        println!();
        println!("  {} Polymarket", "Balance:".green().bold());
        println!();
        println!("  {} {} USDC", "USDC:".dimmed(), balance.to_string().cyan());
        println!();
    }

    Ok(())
}
