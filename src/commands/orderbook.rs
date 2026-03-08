use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::{json, Value};

use crate::{binance, coinbase, config};

/// Spot orderbook: fintool orderbook BTC [--levels 5] [--exchange auto]
pub async fn run_spot(
    symbol: &str,
    levels: usize,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let client = reqwest::Client::new();
    let symbol_upper = symbol.to_uppercase();
    let exchange_name = resolve_exchange_spot(exchange)?;

    let (bids, asks) = match exchange_name.as_str() {
        "hyperliquid" => {
            let coin = resolve_hl_spot_coin(&client, &symbol_upper).await?;
            fetch_hl_orderbook(&client, &coin, levels).await?
        }
        "binance" => fetch_binance_orderbook(&client, &symbol_upper, levels, false).await?,
        "coinbase" => {
            let (api_key, api_secret) = config::coinbase_credentials()
                .ok_or_else(|| anyhow::anyhow!("Coinbase API keys not configured"))?;
            let product_id = format!("{}-USD", symbol_upper);
            fetch_coinbase_orderbook(&client, &api_key, &api_secret, &product_id, levels).await?
        }
        _ => bail!("Unsupported exchange: {}", exchange_name),
    };

    output(
        &symbol_upper,
        "spot",
        &exchange_name,
        &bids,
        &asks,
        json_output,
    )
}

/// Perp orderbook: fintool perp orderbook BTC [--levels 5] [--exchange auto]
pub async fn run_perp(
    symbol: &str,
    levels: usize,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    let client = reqwest::Client::new();
    let symbol_upper = symbol.to_uppercase();
    let exchange_name = resolve_exchange_perp(exchange)?;

    let (bids, asks) = match exchange_name.as_str() {
        "hyperliquid" => fetch_hl_orderbook(&client, &symbol_upper, levels).await?,
        "binance" => fetch_binance_orderbook(&client, &symbol_upper, levels, true).await?,
        _ => bail!("Unsupported exchange for perp orderbook: {}", exchange_name),
    };

    output(
        &symbol_upper,
        "perp",
        &exchange_name,
        &bids,
        &asks,
        json_output,
    )
}

// ── Exchange resolution ─────────────────────────────────────────────────────

fn resolve_exchange_spot(exchange: &str) -> Result<String> {
    match exchange.to_lowercase().as_str() {
        "auto" | "hyperliquid" | "hl" => Ok("hyperliquid".to_string()),
        "binance" => Ok("binance".to_string()),
        "coinbase" | "cb" => Ok("coinbase".to_string()),
        other => bail!("Unknown exchange: {}", other),
    }
}

fn resolve_exchange_perp(exchange: &str) -> Result<String> {
    match exchange.to_lowercase().as_str() {
        "auto" | "hyperliquid" | "hl" => Ok("hyperliquid".to_string()),
        "binance" => Ok("binance".to_string()),
        "coinbase" | "cb" => bail!("Coinbase does not support perpetual futures"),
        other => bail!("Unknown exchange: {}", other),
    }
}

// ── Hyperliquid ─────────────────────────────────────────────────────────────

/// Resolve spot symbol to Hyperliquid @index format (e.g. "BTC" -> "@107")
async fn resolve_hl_spot_coin(client: &reqwest::Client, symbol: &str) -> Result<String> {
    let url = config::info_url();

    let spot_meta: Value = client
        .post(&url)
        .json(&json!({"type": "spotMeta"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse spotMeta")?;

    // Build token index -> name map
    let mut idx_to_name = std::collections::HashMap::new();
    if let Some(tokens) = spot_meta.get("tokens").and_then(|t| t.as_array()) {
        for token in tokens {
            if let (Some(idx), Some(name)) = (
                token.get("index").and_then(|i| i.as_u64()),
                token.get("name").and_then(|n| n.as_str()),
            ) {
                idx_to_name.insert(idx, name.to_string());
            }
        }
    }

    // Find the spot pair where the first token matches our symbol
    if let Some(universe) = spot_meta.get("universe").and_then(|u| u.as_array()) {
        for pair in universe {
            if let (Some(tokens), Some(index)) = (
                pair.get("tokens").and_then(|t| t.as_array()),
                pair.get("index").and_then(|i| i.as_u64()),
            ) {
                if tokens.len() == 2 {
                    let t1 = tokens[0].as_u64().unwrap_or(0);
                    let name1 = idx_to_name.get(&t1).map(|s| s.as_str()).unwrap_or("");
                    if name1.eq_ignore_ascii_case(symbol) {
                        return Ok(format!("@{}", index));
                    }
                }
            }
        }
    }

    bail!("Symbol {} not found on Hyperliquid spot", symbol)
}

/// Fetch L2 orderbook from Hyperliquid (works for both spot @index and perp symbol)
async fn fetch_hl_orderbook(
    client: &reqwest::Client,
    coin: &str,
    levels: usize,
) -> Result<(Vec<Level>, Vec<Level>)> {
    let url = config::info_url();

    let resp: Value = client
        .post(&url)
        .json(&json!({"type": "l2Book", "coin": coin}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse l2Book response")?;

    let levels_arr = resp
        .get("levels")
        .and_then(|l| l.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid l2Book response: missing levels"))?;

    let bids = parse_hl_levels(levels_arr.first(), levels);
    let asks = parse_hl_levels(levels_arr.get(1), levels);

    Ok((bids, asks))
}

fn parse_hl_levels(side: Option<&Value>, max_levels: usize) -> Vec<Level> {
    let Some(arr) = side.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .take(max_levels)
        .filter_map(|entry| {
            let px = entry.get("px")?.as_str()?.to_string();
            let sz = entry.get("sz")?.as_str()?.to_string();
            let n = entry.get("n").and_then(|v| v.as_u64());
            Some(Level {
                price: px,
                size: sz,
                num_orders: n,
            })
        })
        .collect()
}

// ── Binance ─────────────────────────────────────────────────────────────────

async fn fetch_binance_orderbook(
    client: &reqwest::Client,
    symbol: &str,
    levels: usize,
    futures: bool,
) -> Result<(Vec<Level>, Vec<Level>)> {
    let limit = snap_binance_limit(levels);
    let base = if futures {
        binance::FUTURES_BASE_URL
    } else {
        binance::SPOT_BASE_URL
    };
    let endpoint = if futures {
        "/fapi/v1/depth"
    } else {
        "/api/v3/depth"
    };
    let url = format!("{}{}?symbol={}USDT&limit={}", base, endpoint, symbol, limit);

    let resp: Value = client
        .get(&url)
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse Binance depth response")?;

    if let Some(msg) = resp.get("msg").and_then(|m| m.as_str()) {
        bail!("Binance API error: {}", msg);
    }

    let bids = parse_binance_levels(resp.get("bids"), levels);
    let asks = parse_binance_levels(resp.get("asks"), levels);

    Ok((bids, asks))
}

fn parse_binance_levels(side: Option<&Value>, max_levels: usize) -> Vec<Level> {
    let Some(arr) = side.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .take(max_levels)
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let px = pair.first()?.as_str()?.to_string();
            let sz = pair.get(1)?.as_str()?.to_string();
            Some(Level {
                price: px,
                size: sz,
                num_orders: None,
            })
        })
        .collect()
}

/// Snap user-requested levels to a valid Binance limit value
fn snap_binance_limit(levels: usize) -> usize {
    for &valid in &[5, 10, 20, 50, 100, 500, 1000] {
        if levels <= valid {
            return valid;
        }
    }
    1000
}

// ── Coinbase ────────────────────────────────────────────────────────────────

async fn fetch_coinbase_orderbook(
    client: &reqwest::Client,
    api_key: &str,
    api_secret: &str,
    product_id: &str,
    levels: usize,
) -> Result<(Vec<Level>, Vec<Level>)> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let path = format!(
        "/api/v3/brokerage/market/products/{}/book?limit={}",
        product_id, levels
    );
    let signature = coinbase::sign_request(api_secret, &ts, "GET", &path, "");
    let url = format!("https://api.coinbase.com{}", path);

    let resp: Value = client
        .get(&url)
        .header("CB-ACCESS-KEY", api_key)
        .header("CB-ACCESS-SIGN", signature)
        .header("CB-ACCESS-TIMESTAMP", &ts)
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse Coinbase book response")?;

    // Coinbase wraps in {"pricebook": {"bids": [...], "asks": [...]}}
    let book = resp.get("pricebook").unwrap_or(&resp);

    let bids = parse_coinbase_levels(book.get("bids"), levels);
    let asks = parse_coinbase_levels(book.get("asks"), levels);

    Ok((bids, asks))
}

fn parse_coinbase_levels(side: Option<&Value>, max_levels: usize) -> Vec<Level> {
    let Some(arr) = side.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .take(max_levels)
        .filter_map(|entry| {
            let px = entry.get("price")?.as_str()?.to_string();
            let sz = entry.get("size")?.as_str()?.to_string();
            Some(Level {
                price: px,
                size: sz,
                num_orders: None,
            })
        })
        .collect()
}

// ── Data types ──────────────────────────────────────────────────────────────

struct Level {
    price: String,
    size: String,
    num_orders: Option<u64>,
}

// ── Output ──────────────────────────────────────────────────────────────────

fn output(
    symbol: &str,
    market: &str,
    exchange: &str,
    bids: &[Level],
    asks: &[Level],
    json_output: bool,
) -> Result<()> {
    if bids.is_empty() && asks.is_empty() {
        bail!("No orderbook data returned for {}", symbol);
    }

    let sp = compute_spread(bids, asks);

    if json_output {
        let mut result = json!({
            "symbol": symbol,
            "market": market,
            "exchange": exchange,
            "bids": bids.iter().map(|l| {
                let mut m = json!({"price": l.price, "size": l.size});
                if let Some(n) = l.num_orders { m["numOrders"] = json!(n); }
                m
            }).collect::<Vec<_>>(),
            "asks": asks.iter().map(|l| {
                let mut m = json!({"price": l.price, "size": l.size});
                if let Some(n) = l.num_orders { m["numOrders"] = json!(n); }
                m
            }).collect::<Vec<_>>(),
        });
        if let Some(s) = &sp {
            result["spread"] = json!(format!("{:.4}", s.spread));
            result["spreadPct"] = json!(format!("{:.4}", s.spread_pct));
            result["midPrice"] = json!(format!("{:.4}", s.mid_price));
        }
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Human-readable output
    let exchange_label = match exchange {
        "hyperliquid" => "Hyperliquid",
        "binance" => "Binance",
        "coinbase" => "Coinbase",
        other => other,
    };

    println!(
        "\n  {} {} ({})  [{}]\n",
        "Orderbook:".bold(),
        symbol.cyan(),
        market,
        exchange_label
    );

    // Column widths
    let pw = 14; // price column
    let sw = 14; // size column

    // Header
    println!(
        "  {:>sw$}  {:>pw$}  {:<pw$}  {:<sw$}",
        "Bid Size".dimmed(),
        "Bid Price".dimmed(),
        "Ask Price".dimmed(),
        "Ask Size".dimmed(),
        sw = sw,
        pw = pw,
    );
    println!("  {}", "─".repeat(sw + pw + pw + sw + 6).dimmed());

    let rows = std::cmp::max(bids.len(), asks.len());
    for i in 0..rows {
        let bid_sz = bids.get(i).map(|l| l.size.as_str()).unwrap_or("");
        let bid_px = bids.get(i).map(|l| l.price.as_str()).unwrap_or("");
        let ask_px = asks.get(i).map(|l| l.price.as_str()).unwrap_or("");
        let ask_sz = asks.get(i).map(|l| l.size.as_str()).unwrap_or("");

        println!(
            "  {:>sw$}  {:>pw$}  {:<pw$}  {:<sw$}",
            bid_sz,
            bid_px.green(),
            ask_px.red(),
            ask_sz,
            sw = sw,
            pw = pw,
        );
    }

    if let Some(s) = &sp {
        println!(
            "\n  Spread: ${:.4} ({:.4}%)   Mid: ${:.4}\n",
            s.spread, s.spread_pct, s.mid_price
        );
    }

    Ok(())
}

struct Spread {
    spread: f64,
    spread_pct: f64,
    mid_price: f64,
}

fn compute_spread(bids: &[Level], asks: &[Level]) -> Option<Spread> {
    let best_bid: f64 = bids.first()?.price.parse().ok()?;
    let best_ask: f64 = asks.first()?.price.parse().ok()?;

    if best_bid == 0.0 || best_ask == 0.0 {
        return None;
    }

    let spread = best_ask - best_bid;
    let mid = (best_bid + best_ask) / 2.0;
    let pct = spread / mid * 100.0;

    Some(Spread {
        spread,
        spread_pct: pct,
        mid_price: mid,
    })
}
