use anyhow::Result;
use colored::Colorize;
use serde_json::{json, Value};
use tabled::{Table, settings::Style, Tabled};

use crate::config;
use crate::format;

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

    let positions = resp["assetPositions"]
        .as_array()
        .cloned()
        .unwrap_or_default();

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

    let rows: Vec<PositionRow> = active.iter().map(|p| {
        let pos = &p["position"];
        let szi = pos["szi"].as_str().unwrap_or("0");
        let is_long = !szi.starts_with('-');
        PositionRow {
            symbol: pos["coin"].as_str().unwrap_or("").to_string(),
            side: if is_long { "LONG".green().to_string() } else { "SHORT".red().to_string() },
            size: szi.to_string(),
            entry: format!("${}", pos["entryPx"].as_str().unwrap_or("-")),
            mark: format!("${}", pos["positionValue"].as_str().unwrap_or("-")),
            pnl: format::color_pnl(pos["unrealizedPnl"].as_str().unwrap_or("0")),
            leverage: format!("{}x", pos["leverage"]["value"].as_str()
                .or_else(|| pos["leverage"]["value"].as_f64().map(|_| ""))
                .unwrap_or("-")),
        }
    }).collect();

    println!("\n  {} Open Positions\n", "📊");
    let table = Table::new(rows).with(Style::rounded()).to_string();
    for line in table.lines() {
        println!("  {}", line);
    }
    println!();

    Ok(())
}
