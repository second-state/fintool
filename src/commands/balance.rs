use anyhow::Result;
use colored::Colorize;
use serde_json::{json, Value};
use tabled::{settings::Style, Table, Tabled};

use crate::config;

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

pub async fn run(json_output: bool) -> Result<()> {
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
