use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::commands::quote::{coingecko_symbol_map, symbol_aliases};
use crate::format::{color_change, color_pnl};

// ── Data types ──────────────────────────────────────────────────────────

/// A single daily OHLCV bar.
#[derive(Debug, Clone, Serialize)]
pub struct DailyBar {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// A simulated trade record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimTrade {
    pub id: usize,
    pub symbol: String,
    pub side: TradeSide,
    pub amount: f64,
    pub price: f64,
    pub date: String,
    pub trade_type: TradeType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TradeType {
    Spot,
    Perp,
}

// ── Portfolio ───────────────────────────────────────────────────────────

/// A computed position for a (symbol, trade_type) pair.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub symbol: String,
    pub trade_type: TradeType,
    pub net_quantity: f64,
    pub avg_entry_price: f64,
    pub total_cost: f64,
    pub side: String,
}

/// Portfolio state for backtesting. Persisted to ~/.fintool/backtest_portfolio.json.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portfolio {
    pub trades: Vec<SimTrade>,
    pub leverage_settings: HashMap<String, u32>,
    next_id: usize,
}

impl Portfolio {
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
            leverage_settings: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add_trade(
        &mut self,
        symbol: &str,
        side: TradeSide,
        amount: f64,
        price: f64,
        date: NaiveDate,
        trade_type: TradeType,
    ) -> SimTrade {
        let trade = SimTrade {
            id: self.next_id,
            symbol: symbol.to_uppercase(),
            side,
            amount,
            price,
            date: date.to_string(),
            trade_type,
        };
        self.next_id += 1;
        self.trades.push(trade.clone());
        trade
    }

    pub fn set_leverage(&mut self, symbol: &str, leverage: u32) {
        self.leverage_settings
            .insert(symbol.to_uppercase(), leverage);
    }

    pub fn get_leverage(&self, symbol: &str) -> u32 {
        *self
            .leverage_settings
            .get(&symbol.to_uppercase())
            .unwrap_or(&1)
    }

    /// Load portfolio from disk, or return a fresh one if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = portfolio_path();
        if !path.exists() {
            return Ok(Self::new());
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read portfolio file: {}", path.display()))?;
        let portfolio: Portfolio = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse portfolio file: {}", path.display()))?;
        Ok(portfolio)
    }

    /// Save portfolio to disk.
    pub fn save(&self) -> Result<()> {
        let path = portfolio_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write portfolio file: {}", path.display()))?;
        Ok(())
    }

    /// Reset the portfolio (clear all trades and leverage, reset next_id).
    pub fn reset(&mut self) {
        self.trades.clear();
        self.leverage_settings.clear();
        self.next_id = 1;
    }

    /// Compute cash balance from spot trades. Buy subtracts, sell adds.
    pub fn cash_balance(&self) -> f64 {
        let balance: f64 = self
            .trades
            .iter()
            .filter(|t| t.trade_type == TradeType::Spot)
            .map(|t| match t.side {
                TradeSide::Buy => -(t.amount * t.price),
                TradeSide::Sell => t.amount * t.price,
            })
            .sum();
        if balance == 0.0 {
            0.0
        } else {
            balance
        }
    }

    /// Compute net positions grouped by (symbol, trade_type).
    pub fn positions(&self) -> Vec<Position> {
        let mut groups: BTreeMap<(String, TradeType), Vec<&SimTrade>> = BTreeMap::new();
        for trade in &self.trades {
            groups
                .entry((trade.symbol.clone(), trade.trade_type))
                .or_default()
                .push(trade);
        }

        let mut positions = Vec::new();
        for ((symbol, trade_type), trades) in &groups {
            let mut net_qty: f64 = 0.0;
            let mut total_cost: f64 = 0.0;
            // Track long cost basis
            let mut long_cost: f64 = 0.0;
            let mut long_qty: f64 = 0.0;
            // Track short cost basis
            let mut short_cost: f64 = 0.0;
            let mut short_qty: f64 = 0.0;

            for t in trades {
                match t.side {
                    TradeSide::Buy => {
                        net_qty += t.amount;
                        total_cost -= t.amount * t.price;
                        long_cost += t.amount * t.price;
                        long_qty += t.amount;
                        // Reduce short basis if closing a short
                        if short_qty > 0.0 {
                            let avg = short_cost / short_qty;
                            let reduce = t.amount.min(short_qty);
                            short_cost -= reduce * avg;
                            short_qty -= reduce;
                        }
                    }
                    TradeSide::Sell => {
                        net_qty -= t.amount;
                        total_cost += t.amount * t.price;
                        short_cost += t.amount * t.price;
                        short_qty += t.amount;
                        // Reduce long basis if closing a long
                        if long_qty > 0.0 {
                            let avg = long_cost / long_qty;
                            let reduce = t.amount.min(long_qty);
                            long_cost -= reduce * avg;
                            long_qty -= reduce;
                        }
                    }
                }
            }

            let avg_entry = if net_qty > 1e-12 && long_qty > 0.0 {
                long_cost / long_qty
            } else if net_qty < -1e-12 && short_qty > 0.0 {
                short_cost / short_qty
            } else {
                0.0
            };

            let side = if net_qty > 1e-12 {
                "long".to_string()
            } else if net_qty < -1e-12 {
                "short".to_string()
            } else {
                continue; // skip flat positions
            };

            positions.push(Position {
                symbol: symbol.clone(),
                trade_type: *trade_type,
                net_quantity: net_qty,
                avg_entry_price: avg_entry,
                total_cost,
                side,
            });
        }
        positions
    }

    /// Total number of trades recorded.
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }
}

/// Return the portfolio state file path (~/.fintool/backtest_portfolio.json).
fn portfolio_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".fintool")
        .join("backtest_portfolio.json")
}

// ── Yahoo Finance historical data ───────────────────────────────────────

/// Resolve a symbol to Yahoo Finance ticker candidates.
fn resolve_yahoo_tickers(symbol: &str) -> Vec<String> {
    let upper = symbol.to_uppercase();
    let aliases = symbol_aliases();
    let resolved = aliases
        .get(upper.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| upper.clone());

    // For known crypto symbols, try -USD suffix first
    let is_crypto = coingecko_symbol_map().contains_key(upper.as_str());
    if is_crypto {
        vec![format!("{}-USD", upper), resolved]
    } else {
        vec![resolved]
    }
}

/// Fetch daily OHLCV bars from Yahoo Finance for a date range.
pub async fn fetch_yahoo_bars(
    symbol: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyBar>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;

    let tickers = resolve_yahoo_tickers(symbol);
    let period1 = from.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let period2 = to.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();

    for ticker in &tickers {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?period1={}&period2={}&interval=1d",
            ticker, period1, period2
        );
        let resp = client.get(&url).send().await;
        let resp = match resp {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };
        let body: Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let result = &body["chart"]["result"];
        if result.is_null() || !result.is_array() {
            continue;
        }
        let result = &result[0];
        let timestamps = match result["timestamp"].as_array() {
            Some(ts) => ts,
            None => continue,
        };
        let quote = &result["indicators"]["quote"][0];
        let opens = quote["open"].as_array();
        let highs = quote["high"].as_array();
        let lows = quote["low"].as_array();
        let closes = quote["close"].as_array();
        let volumes = quote["volume"].as_array();

        if opens.is_none() || closes.is_none() {
            continue;
        }
        let opens = opens.unwrap();
        let highs = highs.unwrap();
        let lows = lows.unwrap();
        let closes = closes.unwrap();
        let volumes = volumes.unwrap();

        let mut bars = Vec::new();
        for i in 0..timestamps.len() {
            let ts = timestamps[i].as_i64().unwrap_or(0);
            let date = chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.date_naive())
                .unwrap_or(from);

            let open = opens[i].as_f64().unwrap_or(0.0);
            let close = closes[i].as_f64().unwrap_or(0.0);
            if close == 0.0 {
                continue; // skip empty bars
            }

            bars.push(DailyBar {
                date,
                open,
                high: highs[i].as_f64().unwrap_or(open),
                low: lows[i].as_f64().unwrap_or(open),
                close,
                volume: volumes[i].as_f64().unwrap_or(0.0),
            });
        }

        if !bars.is_empty() {
            bars.sort_by_key(|b| b.date);
            return Ok(bars);
        }
    }
    anyhow::bail!("No historical data found for '{}' on Yahoo Finance", symbol)
}

// ── CoinGecko historical data ───────────────────────────────────────────

/// Fetch a single historical price from CoinGecko.
pub async fn fetch_coingecko_price(symbol: &str, date: NaiveDate) -> Result<f64> {
    let map = coingecko_symbol_map();
    let upper = symbol.to_uppercase();
    let id = map
        .get(upper.as_str())
        .ok_or_else(|| anyhow::anyhow!("Unknown CoinGecko symbol: {}", symbol))?;
    let date_str = date.format("%d-%m-%Y").to_string();
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/{}/history?date={}&localization=false",
        id, date_str
    );
    let client = reqwest::Client::builder()
        .user_agent("fintool/0.1")
        .build()?;
    let resp: Value = client
        .get(&url)
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse CoinGecko response")?;

    resp["market_data"]["current_price"]["usd"]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("No price data from CoinGecko for {} on {}", symbol, date))
}

// ── Convenience functions ───────────────────────────────────────────────

/// Fetch the historical close price at a specific date.
/// Tries Yahoo Finance first, falls back to CoinGecko for crypto.
pub async fn fetch_price_at_date(symbol: &str, date: NaiveDate) -> Result<f64> {
    // Fetch a range around the date to handle weekends/holidays
    let from = date - chrono::Duration::days(7);
    if let Ok(bars) = fetch_yahoo_bars(symbol, from, date).await {
        // Find the bar closest to (but not after) the target date
        if let Some(bar) = bars.iter().rev().find(|b| b.date <= date) {
            return Ok(bar.close);
        }
    }
    // Fallback to CoinGecko
    fetch_coingecko_price(symbol, date).await
}

/// Fetch prices at multiple future offsets for PnL calculation.
/// Returns Vec of (label, target_date, price_if_available).
pub async fn fetch_pnl_prices(
    symbol: &str,
    trade_date: NaiveDate,
) -> Result<Vec<(String, NaiveDate, Option<f64>)>> {
    let offsets: &[(i64, &str)] = &[
        (1, "+1 day"),
        (2, "+2 days"),
        (4, "+4 days"),
        (7, "+7 days"),
    ];

    // Fetch a wide range of bars (trade_date to +10 days buffer for weekends)
    let end_date = trade_date + chrono::Duration::days(12);
    let bars = fetch_yahoo_bars(symbol, trade_date, end_date).await;

    let mut results = Vec::new();
    for (days, label) in offsets {
        let target = trade_date + chrono::Duration::days(*days);
        let price = match &bars {
            Ok(bars) => {
                // Find closest bar on or after target date (handles weekends)
                bars.iter().find(|b| b.date >= target).map(|b| b.close)
            }
            Err(_) => None,
        };
        results.push((label.to_string(), target, price));
    }
    Ok(results)
}

// ── PnL display ─────────────────────────────────────────────────────────

/// Build PnL JSON for a simulated trade (used by JSON mode).
pub fn build_pnl_json(
    trade: &SimTrade,
    future_prices: &[(String, NaiveDate, Option<f64>)],
    leverage: u32,
) -> Value {
    let multiplier = if trade.side == TradeSide::Buy {
        1.0
    } else {
        -1.0
    };

    let pnl_entries: Vec<Value> = future_prices
        .iter()
        .map(|(label, date, price)| match price {
            Some(p) => {
                let raw_pnl = (p - trade.price) * trade.amount * multiplier;
                let leveraged_pnl = raw_pnl * leverage as f64;
                let entry_cost = trade.amount * trade.price;
                let pct = if entry_cost > 0.0 {
                    (leveraged_pnl / entry_cost) * 100.0
                } else {
                    0.0
                };
                json!({
                    "offset": label,
                    "date": date.to_string(),
                    "price": format!("{:.2}", p),
                    "pnl": format!("{:.2}", leveraged_pnl),
                    "pnlPct": format!("{:.2}", pct),
                })
            }
            None => json!({
                "offset": label,
                "date": date.to_string(),
                "price": null,
                "pnl": null,
                "pnlPct": null,
            }),
        })
        .collect();

    json!({
        "trade": {
            "id": trade.id,
            "symbol": trade.symbol,
            "side": trade.side,
            "amount": trade.amount,
            "price": trade.price,
            "date": trade.date,
            "type": trade.trade_type,
            "leverage": leverage,
        },
        "pnl": pnl_entries,
    })
}

/// Build portfolio summary JSON.
pub fn build_portfolio_json(portfolio: &Portfolio) -> Value {
    let cash = portfolio.cash_balance();
    let positions = portfolio.positions();
    let pos_json: Vec<Value> = positions
        .iter()
        .map(|p| {
            json!({
                "symbol": p.symbol,
                "type": p.trade_type,
                "side": p.side,
                "quantity": p.net_quantity,
                "avgEntryPrice": format!("{:.2}", p.avg_entry_price),
            })
        })
        .collect();
    json!({
        "cashBalance": format!("{:.2}", cash),
        "positions": pos_json,
        "totalTrades": portfolio.trade_count(),
    })
}

/// Print a PnL table for a simulated trade (human-readable CLI mode).
pub fn print_pnl_table(
    trade: &SimTrade,
    future_prices: &[(String, NaiveDate, Option<f64>)],
    leverage: u32,
) -> Result<()> {
    let multiplier = if trade.side == TradeSide::Buy {
        1.0
    } else {
        -1.0
    };

    // Human-readable output
    use colored::Colorize;

    let side_str = match trade.side {
        TradeSide::Buy => "BUY",
        TradeSide::Sell => "SELL",
    };
    let type_str = match trade.trade_type {
        TradeType::Spot => "spot",
        TradeType::Perp => "perp",
    };
    let total = trade.amount * trade.price;

    println!();
    println!(
        "  {} {} {} {} @ ${:.2} (${:.2} total) on {}{}",
        "[BACKTEST]".dimmed(),
        type_str,
        side_str.bold(),
        trade.symbol.cyan(),
        trade.price,
        total,
        trade.date,
        if leverage > 1 {
            format!(" [{}x leverage]", leverage)
        } else {
            String::new()
        }
    );
    println!();

    // Build table
    let header: Vec<String> = future_prices.iter().map(|(l, _, _)| l.clone()).collect();
    let mut dollar_vals = Vec::new();
    let mut pct_vals = Vec::new();
    let mut price_vals = Vec::new();

    for (_, _, price) in future_prices {
        match price {
            Some(p) => {
                let raw_pnl = (p - trade.price) * trade.amount * multiplier;
                let leveraged_pnl = raw_pnl * leverage as f64;
                let entry_cost = trade.amount * trade.price;
                let pct = if entry_cost > 0.0 {
                    (leveraged_pnl / entry_cost) * 100.0
                } else {
                    0.0
                };
                price_vals.push(format!("${:.2}", p));
                dollar_vals.push(color_pnl(&format!("{:.2}", leveraged_pnl)));
                pct_vals.push(color_change(&format!("{:.2}", pct)));
            }
            None => {
                price_vals.push("N/A".to_string());
                dollar_vals.push("N/A".to_string());
                pct_vals.push("N/A".to_string());
            }
        }
    }

    // Manual table — tabled doesn't handle dynamic columns well
    let col_width = 14;
    let label_width = 8;

    // Header
    print!("  {:<label_width$}", "");
    for h in &header {
        print!(" {:>col_width$}", h);
    }
    println!();

    // Separator
    print!("  {:<label_width$}", "");
    for _ in &header {
        print!(" {:>col_width$}", "──────────────");
    }
    println!();

    // Price row
    print!("  {:<label_width$}", "Price");
    for v in &price_vals {
        print!(" {:>col_width$}", v);
    }
    println!();

    // PnL $ row
    print!("  {:<label_width$}", "PnL $");
    for v in &dollar_vals {
        print!(" {:>col_width$}", v);
    }
    println!();

    // PnL % row
    print!("  {:<label_width$}", "PnL %");
    for v in &pct_vals {
        print!(" {:>col_width$}", v);
    }
    println!();
    println!();

    Ok(())
}

/// Print a portfolio summary (human-readable CLI mode).
pub fn print_portfolio_summary(portfolio: &Portfolio) {
    use colored::Colorize;

    let cash = portfolio.cash_balance();
    let positions = portfolio.positions();

    let cash_str = format!("${:.2}", cash);
    let colored_cash = if cash >= 0.0 {
        cash_str.green().to_string()
    } else {
        cash_str.red().to_string()
    };
    println!(
        "  {} Cash balance: {}",
        "[PORTFOLIO]".dimmed(),
        colored_cash
    );
    for p in &positions {
        let type_str = match p.trade_type {
            TradeType::Spot => "spot",
            TradeType::Perp => "perp",
        };
        println!(
            "  {} {} {} {}: {:.4} @ avg ${:.2}",
            "[PORTFOLIO]".dimmed(),
            type_str,
            p.side,
            p.symbol.cyan(),
            p.net_quantity.abs(),
            p.avg_entry_price,
        );
    }
    println!();
}
