use anyhow::Result;
use colored::Colorize;
use serde_json::{json, Value};
use tabled::{settings::Style, Table, Tabled};

use crate::config;

#[derive(Tabled)]
struct OrderRow {
    #[tabled(rename = "OID")]
    oid: String,
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Side")]
    side: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Price")]
    price: String,
    #[tabled(rename = "Type")]
    order_type: String,
}

pub async fn run(symbol: Option<&str>, json_output: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let client = reqwest::Client::new();
    let url = config::info_url();

    let resp: Value = client
        .post(&url)
        .json(&json!({"type": "openOrders", "user": cfg.address}))
        .send()
        .await?
        .json()
        .await?;

    let orders = resp.as_array().cloned().unwrap_or_default();

    // Filter by symbol if provided
    let orders: Vec<&Value> = if let Some(sym) = symbol {
        let sym = sym.to_uppercase();
        orders
            .iter()
            .filter(|o| o.get("coin").and_then(|c| c.as_str()) == Some(sym.as_str()))
            .collect()
    } else {
        orders.iter().collect()
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&orders)?);
        return Ok(());
    }

    if orders.is_empty() {
        println!("\n  No open orders.\n");
        return Ok(());
    }

    let rows: Vec<OrderRow> = orders
        .iter()
        .map(|o| OrderRow {
            oid: o["oid"].as_str().unwrap_or("").chars().take(8).collect(),
            symbol: o["coin"].as_str().unwrap_or("").to_string(),
            side: if o["side"].as_str() == Some("B") {
                "BUY".green().to_string()
            } else {
                "SELL".red().to_string()
            },
            size: o["sz"].as_str().unwrap_or("0").to_string(),
            price: format!("${}", o["limitPx"].as_str().unwrap_or("0")),
            order_type: o["orderType"].as_str().unwrap_or("Limit").to_string(),
        })
        .collect();

    println!("\n  📋 Open Orders\n");
    let table = Table::new(rows).with(Style::rounded()).to_string();
    for line in table.lines() {
        println!("  {}", line);
    }
    println!();

    Ok(())
}
