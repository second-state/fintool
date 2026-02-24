use anyhow::{bail, Result};
use colored::Colorize;
use serde_json::{json, Value};
use tabled::{settings::Style, Table, Tabled};

use crate::{binance, coinbase, config};

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

pub async fn run(symbol: Option<&str>, exchange: &str, json_output: bool) -> Result<()> {
    let exchange = resolve_exchange(exchange)?;

    if exchange == "coinbase" {
        let (api_key, api_secret) = config::coinbase_credentials()
            .ok_or_else(|| anyhow::anyhow!("Coinbase API credentials not configured"))?;

        let client = reqwest::Client::new();
        let orders =
            coinbase::get_orders(&client, &api_key, &api_secret, symbol, json_output).await?;

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "exchange": "coinbase",
                    "orders": orders,
                }))?
            );
            return Ok(());
        }

        let empty_vec = vec![];
        let order_list = orders
            .get("orders")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        if order_list.is_empty() {
            println!("\n  No open Coinbase orders.\n");
            return Ok(());
        }

        let mut rows = Vec::new();
        for order in order_list {
            let product_id = order
                .get("product_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let order_id = order.get("order_id").and_then(|v| v.as_str()).unwrap_or("");
            let side = order.get("side").and_then(|v| v.as_str()).unwrap_or("");

            // Extract size and price from order_configuration
            let mut size = String::from("0");
            let mut price = String::from("$0");

            if let Some(config) = order.get("order_configuration") {
                if let Some(limit) = config.get("limit_limit_gtc") {
                    size = limit
                        .get("base_size")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string();
                    price = format!(
                        "${}",
                        limit
                            .get("limit_price")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0")
                    );
                }
            }

            rows.push(OrderRow {
                oid: format!("coinbase:{}", order_id),
                symbol: product_id.to_string(),
                side: if side == "BUY" {
                    "BUY".green().to_string()
                } else {
                    "SELL".red().to_string()
                },
                size,
                price,
                order_type: "Spot Limit".to_string(),
            });
        }

        println!("\n  📋 Coinbase Open Orders\n");
        let table = Table::new(rows).with(Style::rounded()).to_string();
        for line in table.lines() {
            println!("  {}", line);
        }
        println!();

        return Ok(());
    }

    if exchange == "binance" {
        let (api_key, api_secret) = config::binance_credentials()
            .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured"))?;

        let client = reqwest::Client::new();

        // Get spot and futures orders
        let spot_symbol = symbol.map(|s| format!("{}USDT", s.to_uppercase()));
        let spot_orders =
            binance::get_spot_open_orders(&client, &api_key, &api_secret, spot_symbol.as_deref())
                .await?;

        let futures_orders = binance::get_futures_open_orders(
            &client,
            &api_key,
            &api_secret,
            spot_symbol.as_deref(),
        )
        .await?;

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "exchange": "binance",
                    "spot": spot_orders,
                    "futures": futures_orders,
                }))?
            );
            return Ok(());
        }

        let mut all_rows = Vec::new();

        // Process spot orders
        if let Some(orders) = spot_orders.as_array() {
            for order in orders {
                let symbol = order.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                let order_id = order.get("orderId").and_then(|v| v.as_u64()).unwrap_or(0);
                all_rows.push(OrderRow {
                    oid: format!("binance_spot:{}:{}", symbol, order_id),
                    symbol: symbol.to_string(),
                    side: if order.get("side").and_then(|v| v.as_str()) == Some("BUY") {
                        "BUY".green().to_string()
                    } else {
                        "SELL".red().to_string()
                    },
                    size: order
                        .get("origQty")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    price: format!(
                        "${}",
                        order.get("price").and_then(|v| v.as_str()).unwrap_or("0")
                    ),
                    order_type: format!(
                        "Spot {}",
                        order
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("LIMIT")
                    ),
                });
            }
        }

        // Process futures orders
        if let Some(orders) = futures_orders.as_array() {
            for order in orders {
                let symbol = order.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                let order_id = order.get("orderId").and_then(|v| v.as_u64()).unwrap_or(0);
                all_rows.push(OrderRow {
                    oid: format!("binance_futures:{}:{}", symbol, order_id),
                    symbol: symbol.to_string(),
                    side: if order.get("side").and_then(|v| v.as_str()) == Some("BUY") {
                        "BUY".green().to_string()
                    } else {
                        "SELL".red().to_string()
                    },
                    size: order
                        .get("origQty")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0")
                        .to_string(),
                    price: format!(
                        "${}",
                        order.get("price").and_then(|v| v.as_str()).unwrap_or("0")
                    ),
                    order_type: format!(
                        "Futures {}",
                        order
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("LIMIT")
                    ),
                });
            }
        }

        if all_rows.is_empty() {
            println!("\n  No open Binance orders.\n");
            return Ok(());
        }

        println!("\n  📋 Binance Open Orders\n");
        let table = Table::new(all_rows).with(Style::rounded()).to_string();
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
