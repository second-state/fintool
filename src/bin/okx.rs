use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::Deserialize;
use serde_json::json;

use fintool_lib::{config, format::fmt_num, okx, resolve_withdraw_destination};

#[derive(Parser)]
#[command(name = "okx", about = "OKX trading CLI — spot, perpetual, and options")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// JSON mode: pass a JSON command string for programmatic use (always outputs JSON).
    /// Example: okx --json '{"command":"buy","symbol":"BTC","amount":0.001,"price":60000}'
    #[arg(long)]
    json: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Place a spot limit buy order
    Buy {
        symbol: String,
        /// Amount of the asset to buy (in symbol units)
        #[arg(long)]
        amount: String,
        /// Maximum price per unit (limit price)
        #[arg(long)]
        price: String,
    },

    /// Place a spot limit sell order
    Sell {
        symbol: String,
        /// Amount of the asset to sell (in symbol units)
        #[arg(long)]
        amount: String,
        /// Minimum price per unit (limit price)
        #[arg(long)]
        price: String,
    },

    /// Perpetual futures trading
    #[command(subcommand)]
    Perp(PerpCmd),

    /// Show L2 orderbook / market depth for a spot pair
    Orderbook {
        symbol: String,
        /// Number of price levels per side (default: 5)
        #[arg(long, default_value = "5")]
        levels: usize,
    },

    /// List open orders (spot and swap)
    Orders { symbol: Option<String> },

    /// Cancel an order
    Cancel {
        /// Instrument ID (e.g. BTC-USDT or BTC-USDT-SWAP)
        #[arg(long)]
        inst_id: String,
        /// Order ID to cancel
        order_id: String,
    },

    /// Show account balances (trading + funding)
    Balance,

    /// Show open positions
    Positions,

    /// Get deposit address
    Deposit {
        /// Asset: ETH, BTC, USDC, etc.
        asset: String,
        /// Network (e.g. ethereum, base, arbitrum)
        #[arg(long)]
        network: Option<String>,
    },

    /// Withdraw from OKX
    Withdraw {
        /// Asset: ETH, BTC, USDC, etc.
        asset: String,
        /// Amount to withdraw
        #[arg(long)]
        amount: String,
        /// Destination: chain name or address
        #[arg(long)]
        to: Option<String>,
        /// Network (e.g. ethereum, base, arbitrum)
        #[arg(long)]
        network: Option<String>,
        /// Withdrawal fee
        #[arg(long)]
        fee: Option<String>,
    },

    /// Get a price quote for a symbol
    Quote { symbol: String },

    /// Transfer assets between funding and trading accounts
    Transfer {
        /// Asset: USDT, USDC, BTC, ETH, etc.
        asset: String,
        /// Amount to transfer
        #[arg(long)]
        amount: String,
        /// Source account: funding or trading
        #[arg(long)]
        from: String,
        /// Destination account: funding or trading
        #[arg(long)]
        to: String,
    },
}

#[derive(Subcommand)]
enum PerpCmd {
    /// Show L2 orderbook for a perpetual swap
    Orderbook {
        symbol: String,
        /// Number of price levels per side (default: 5)
        #[arg(long, default_value = "5")]
        levels: usize,
    },
    /// Place a perp limit buy (long) order
    Buy {
        symbol: String,
        /// Size in asset units
        #[arg(long)]
        amount: String,
        /// Limit price
        #[arg(long)]
        price: String,
        /// Close position only (reduce-only)
        #[arg(long)]
        close: bool,
    },
    /// Place a perp limit sell (short) order
    Sell {
        symbol: String,
        /// Size in asset units
        #[arg(long)]
        amount: String,
        /// Limit price
        #[arg(long)]
        price: String,
        /// Close position only (reduce-only)
        #[arg(long)]
        close: bool,
    },
    /// Set leverage for a swap instrument
    Leverage {
        symbol: String,
        /// Leverage multiplier (e.g. 5, 10, 20)
        #[arg(long)]
        leverage: u32,
        /// Use cross margin instead of isolated
        #[arg(long)]
        cross: bool,
    },
    /// Get funding rate for a swap instrument
    FundingRate { symbol: String },
}

// ── Helpers ─────────────────────────────────────────────────────────

fn get_credentials() -> Result<(String, String, String)> {
    config::okx_credentials().ok_or_else(|| {
        anyhow::anyhow!(
            "OKX API credentials not configured.\n\
             Add okx_api_key, okx_secret_key, and okx_passphrase to [api_keys] in ~/.fintool/config.toml"
        )
    })
}

/// Map user-friendly account names to OKX account type codes
fn account_type(name: &str) -> Result<&'static str> {
    match name.to_lowercase().as_str() {
        "funding" | "fund" => Ok("6"),
        "trading" | "trade" | "unified" => Ok("18"),
        _ => bail!(
            "Invalid account type '{}'. Use 'funding' or 'trading'",
            name
        ),
    }
}

/// Map user-friendly network names to OKX network for deposit address
fn deposit_network(asset: &str, network: Option<&str>) -> String {
    match network {
        Some(n) => okx::map_chain(asset, n),
        None => {
            // Default: ERC20 for most assets
            let upper = asset.to_uppercase();
            match upper.as_str() {
                "BTC" => "BTC-Bitcoin".to_string(),
                "SOL" => "SOL-Solana".to_string(),
                _ => format!("{}-ERC20", upper),
            }
        }
    }
}

// ── Spot order ──────────────────────────────────────────────────────

async fn spot_buy(symbol: &str, amount: &str, price: &str, json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();
    let inst_id = okx::spot_inst_id(symbol);

    let resp = okx::place_order(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        &inst_id,
        "cash",
        "buy",
        "limit",
        amount,
        Some(price),
        false,
    )
    .await?;

    let data = &resp["data"][0];
    let ord_id = data["ordId"].as_str().unwrap_or("unknown");
    let s_code = data["sCode"].as_str().unwrap_or("0");
    let s_msg = data["sMsg"].as_str().unwrap_or("");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "spot_buy",
                "exchange": "okx",
                "instId": inst_id,
                "size": amount,
                "price": price,
                "orderId": ord_id,
                "status": if s_code == "0" { "submitted" } else { "error" },
                "message": s_msg,
                "response": data,
            }))?
        );
    } else {
        println!();
        if s_code == "0" {
            println!("  {} Spot buy order placed on OKX", "OK".green().bold());
            println!("  Instrument: {}", inst_id.cyan());
            println!("  Size:       {}", amount);
            println!("  Price:      ${}", price);
            println!("  Order ID:   {}", ord_id);
        } else {
            println!("  {} Spot buy failed: {}", "Error".red().bold(), s_msg);
        }
        println!();
    }
    Ok(())
}

async fn spot_sell(symbol: &str, amount: &str, price: &str, json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();
    let inst_id = okx::spot_inst_id(symbol);

    let resp = okx::place_order(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        &inst_id,
        "cash",
        "sell",
        "limit",
        amount,
        Some(price),
        false,
    )
    .await?;

    let data = &resp["data"][0];
    let ord_id = data["ordId"].as_str().unwrap_or("unknown");
    let s_code = data["sCode"].as_str().unwrap_or("0");
    let s_msg = data["sMsg"].as_str().unwrap_or("");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "spot_sell",
                "exchange": "okx",
                "instId": inst_id,
                "size": amount,
                "price": price,
                "orderId": ord_id,
                "status": if s_code == "0" { "submitted" } else { "error" },
                "message": s_msg,
                "response": data,
            }))?
        );
    } else {
        println!();
        if s_code == "0" {
            println!("  {} Spot sell order placed on OKX", "OK".green().bold());
            println!("  Instrument: {}", inst_id.cyan());
            println!("  Size:       {}", amount);
            println!("  Price:      ${}", price);
            println!("  Order ID:   {}", ord_id);
        } else {
            println!("  {} Spot sell failed: {}", "Error".red().bold(), s_msg);
        }
        println!();
    }
    Ok(())
}

// ── Perp order ──────────────────────────────────────────────────────

async fn perp_buy(
    symbol: &str,
    amount: &str,
    price: &str,
    close: bool,
    json_output: bool,
) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();
    let inst_id = okx::swap_inst_id(symbol);

    let resp = okx::place_order(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        &inst_id,
        "cross",
        "buy",
        "limit",
        amount,
        Some(price),
        close,
    )
    .await?;

    let data = &resp["data"][0];
    let ord_id = data["ordId"].as_str().unwrap_or("unknown");
    let s_code = data["sCode"].as_str().unwrap_or("0");
    let s_msg = data["sMsg"].as_str().unwrap_or("");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "perp_buy",
                "exchange": "okx",
                "instId": inst_id,
                "size": amount,
                "price": price,
                "reduceOnly": close,
                "orderId": ord_id,
                "status": if s_code == "0" { "submitted" } else { "error" },
                "message": s_msg,
                "response": data,
            }))?
        );
    } else {
        println!();
        if s_code == "0" {
            println!(
                "  {} Perp buy order placed on OKX{}",
                "OK".green().bold(),
                if close { " (reduce-only)" } else { "" }
            );
            println!("  Instrument: {}", inst_id.cyan());
            println!("  Size:       {}", amount);
            println!("  Price:      ${}", price);
            println!("  Order ID:   {}", ord_id);
        } else {
            println!("  {} Perp buy failed: {}", "Error".red().bold(), s_msg);
        }
        println!();
    }
    Ok(())
}

async fn perp_sell(
    symbol: &str,
    amount: &str,
    price: &str,
    close: bool,
    json_output: bool,
) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();
    let inst_id = okx::swap_inst_id(symbol);

    let resp = okx::place_order(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        &inst_id,
        "cross",
        "sell",
        "limit",
        amount,
        Some(price),
        close,
    )
    .await?;

    let data = &resp["data"][0];
    let ord_id = data["ordId"].as_str().unwrap_or("unknown");
    let s_code = data["sCode"].as_str().unwrap_or("0");
    let s_msg = data["sMsg"].as_str().unwrap_or("");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "perp_sell",
                "exchange": "okx",
                "instId": inst_id,
                "size": amount,
                "price": price,
                "reduceOnly": close,
                "orderId": ord_id,
                "status": if s_code == "0" { "submitted" } else { "error" },
                "message": s_msg,
                "response": data,
            }))?
        );
    } else {
        println!();
        if s_code == "0" {
            println!(
                "  {} Perp sell order placed on OKX{}",
                "OK".green().bold(),
                if close { " (reduce-only)" } else { "" }
            );
            println!("  Instrument: {}", inst_id.cyan());
            println!("  Size:       {}", amount);
            println!("  Price:      ${}", price);
            println!("  Order ID:   {}", ord_id);
        } else {
            println!("  {} Perp sell failed: {}", "Error".red().bold(), s_msg);
        }
        println!();
    }
    Ok(())
}

// ── Leverage ────────────────────────────────────────────────────────

async fn set_leverage(symbol: &str, leverage: u32, cross: bool, json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();
    let inst_id = okx::swap_inst_id(symbol);
    let mgn_mode = if cross { "cross" } else { "isolated" };

    let resp = okx::set_leverage(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        &inst_id,
        leverage,
        mgn_mode,
    )
    .await?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "set_leverage",
                "exchange": "okx",
                "instId": inst_id,
                "leverage": leverage,
                "mgnMode": mgn_mode,
                "response": resp["data"],
            }))?
        );
    } else {
        println!();
        println!(
            "  {} Leverage set to {}x for {} ({})",
            "OK".green().bold(),
            leverage,
            inst_id.cyan(),
            mgn_mode
        );
        println!();
    }
    Ok(())
}

// ── Orderbook ───────────────────────────────────────────────────────

async fn orderbook(inst_id: &str, levels: usize, json_output: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = okx::get_orderbook(&client, inst_id, levels).await?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "okx",
                "instId": inst_id,
                "orderbook": resp["data"],
            }))?
        );
        return Ok(());
    }

    let data = &resp["data"][0];
    let asks = data["asks"].as_array();
    let bids = data["bids"].as_array();

    println!();
    println!("  {} {} on OKX", "Orderbook".green().bold(), inst_id.cyan());
    println!();

    // Asks (reversed so lowest ask is at bottom)
    if let Some(asks) = asks {
        println!("  {:>12}  {:>14}", "Ask Price".red(), "Size".dimmed());
        for ask in asks.iter().rev().take(levels) {
            let price = ask[0].as_str().unwrap_or("?");
            let size = ask[1].as_str().unwrap_or("?");
            println!("  {:>12}  {:>14}", price.red(), size);
        }
    }

    println!("  {}", "─".repeat(28).dimmed());

    // Bids
    if let Some(bids) = bids {
        println!("  {:>12}  {:>14}", "Bid Price".green(), "Size".dimmed());
        for bid in bids.iter().take(levels) {
            let price = bid[0].as_str().unwrap_or("?");
            let size = bid[1].as_str().unwrap_or("?");
            println!("  {:>12}  {:>14}", price.green(), size);
        }
    }

    println!();
    Ok(())
}

// ── Balance ─────────────────────────────────────────────────────────

async fn balance(json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();

    let trading = okx::get_balance(&client, &api_key, &api_secret, &passphrase).await?;
    let funding = okx::get_funding_balance(&client, &api_key, &api_secret, &passphrase).await?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "okx",
                "trading": trading["data"],
                "funding": funding["data"],
            }))?
        );
        return Ok(());
    }

    println!();
    println!("  {} OKX Account Balance", "Balance".green().bold());
    println!();

    // Trading account
    println!("  {} Trading Account:", "Trading".cyan().bold());
    if let Some(details) = trading["data"][0]["details"].as_array() {
        for d in details {
            let ccy = d["ccy"].as_str().unwrap_or("?");
            let eq: f64 = d["eq"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let avail: f64 = d["availBal"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let frozen: f64 = d["frozenBal"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            if eq > 0.0 || avail > 0.0 {
                println!(
                    "    {}: {} (available: {}, frozen: {})",
                    ccy.cyan(),
                    eq,
                    avail,
                    frozen
                );
            }
        }
    }

    println!();
    println!("  {} Funding Account:", "Funding".cyan().bold());
    if let Some(balances) = funding["data"].as_array() {
        for b in balances {
            let ccy = b["ccy"].as_str().unwrap_or("?");
            let bal: f64 = b["bal"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let avail: f64 = b["availBal"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            if bal > 0.0 {
                println!("    {}: {} (available: {})", ccy.cyan(), bal, avail);
            }
        }
    }

    println!();
    Ok(())
}

// ── Positions ───────────────────────────────────────────────────────

async fn positions(json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();

    let resp = okx::get_positions(&client, &api_key, &api_secret, &passphrase).await?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "okx",
                "positions": resp["data"],
            }))?
        );
        return Ok(());
    }

    println!();
    println!("  {} OKX Open Positions", "Positions".green().bold());
    println!();

    if let Some(positions) = resp["data"].as_array() {
        if positions.is_empty() {
            println!("  (no open positions)");
        } else {
            for p in positions {
                let inst_id = p["instId"].as_str().unwrap_or("?");
                let pos = p["pos"].as_str().unwrap_or("0");
                let avg_px = p["avgPx"].as_str().unwrap_or("?");
                let mark_px = p["markPx"].as_str().unwrap_or("?");
                let upl = p["upl"].as_str().unwrap_or("0");
                let lever = p["lever"].as_str().unwrap_or("?");
                let pos_side = p["posSide"].as_str().unwrap_or("net");
                let mgn_mode = p["mgnMode"].as_str().unwrap_or("?");

                let upl_f: f64 = upl.parse().unwrap_or(0.0);
                let upl_colored = if upl_f >= 0.0 {
                    format!("+{}", upl).green()
                } else {
                    upl.to_string().red()
                };

                println!(
                    "  {} | {} {} | entry: {} | mark: {} | PnL: {} | {}x {}",
                    inst_id.cyan(),
                    pos_side,
                    pos,
                    avg_px,
                    mark_px,
                    upl_colored,
                    lever,
                    mgn_mode
                );
            }
        }
    }

    println!();
    Ok(())
}

// ── Orders ──────────────────────────────────────────────────────────

async fn orders(inst_type: Option<&str>, json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();

    let resp =
        okx::get_pending_orders(&client, &api_key, &api_secret, &passphrase, inst_type).await?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "okx",
                "orders": resp["data"],
            }))?
        );
        return Ok(());
    }

    println!();
    println!("  {} OKX Open Orders", "Orders".green().bold());
    println!();

    if let Some(orders) = resp["data"].as_array() {
        if orders.is_empty() {
            println!("  (no open orders)");
        } else {
            for o in orders {
                let inst_id = o["instId"].as_str().unwrap_or("?");
                let side = o["side"].as_str().unwrap_or("?");
                let sz = o["sz"].as_str().unwrap_or("?");
                let px = o["px"].as_str().unwrap_or("?");
                let ord_type = o["ordType"].as_str().unwrap_or("?");
                let ord_id = o["ordId"].as_str().unwrap_or("?");
                let state = o["state"].as_str().unwrap_or("?");

                let side_colored = if side == "buy" {
                    side.green()
                } else {
                    side.red()
                };

                println!(
                    "  {} | {} {} @ ${} | {} | {} | {}",
                    inst_id.cyan(),
                    side_colored,
                    sz,
                    px,
                    ord_type,
                    state,
                    ord_id.dimmed()
                );
            }
        }
    }

    println!();
    Ok(())
}

// ── Cancel ──────────────────────────────────────────────────────────

async fn cancel(inst_id: &str, order_id: &str, json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();

    let resp = okx::cancel_order(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        inst_id,
        order_id,
    )
    .await?;

    let data = &resp["data"][0];
    let s_code = data["sCode"].as_str().unwrap_or("0");
    let s_msg = data["sMsg"].as_str().unwrap_or("");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "cancel",
                "exchange": "okx",
                "instId": inst_id,
                "orderId": order_id,
                "status": if s_code == "0" { "cancelled" } else { "error" },
                "message": s_msg,
                "response": data,
            }))?
        );
    } else {
        println!();
        if s_code == "0" {
            println!(
                "  {} Order {} cancelled on {}",
                "OK".green().bold(),
                order_id,
                inst_id.cyan()
            );
        } else {
            println!("  {} Cancel failed: {}", "Error".red().bold(), s_msg);
        }
        println!();
    }
    Ok(())
}

// ── Quote ───────────────────────────────────────────────────────────

async fn quote(symbol: &str, json_output: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let inst_id = okx::spot_inst_id(symbol);

    let resp = okx::get_ticker(&client, &inst_id).await?;

    let data = &resp["data"][0];
    let last = data["last"].as_str().unwrap_or("?");
    let bid = data["bidPx"].as_str().unwrap_or("?");
    let ask = data["askPx"].as_str().unwrap_or("?");
    let vol24h = data["vol24h"].as_str().unwrap_or("?");
    let high24h = data["high24h"].as_str().unwrap_or("?");
    let low24h = data["low24h"].as_str().unwrap_or("?");
    let open24h = data["open24h"].as_str().unwrap_or("?");

    // Calculate 24h change
    let last_f: f64 = last.parse().unwrap_or(0.0);
    let open_f: f64 = open24h.parse().unwrap_or(0.0);
    let change_pct = if open_f > 0.0 {
        (last_f - open_f) / open_f * 100.0
    } else {
        0.0
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "okx",
                "symbol": symbol.to_uppercase(),
                "instId": inst_id,
                "price": last,
                "bid": bid,
                "ask": ask,
                "open24h": open24h,
                "high24h": high24h,
                "low24h": low24h,
                "volume24h": vol24h,
                "change24hPct": format!("{:.2}", change_pct),
            }))?
        );
    } else {
        println!();
        println!(
            "  {} {} on OKX",
            symbol.to_uppercase().cyan().bold(),
            "Quote".green().bold()
        );
        println!();
        println!("  Price:      ${}", last);
        println!("  Bid/Ask:    ${} / ${}", bid, ask);
        let change_str = format!("{:+.2}%", change_pct);
        if change_pct >= 0.0 {
            println!("  24h Change: {}", change_str.green());
        } else {
            println!("  24h Change: {}", change_str.red());
        }
        println!("  24h Range:  ${} — ${}", low24h, high24h);
        println!("  24h Volume: {}", vol24h);
        println!();
    }
    Ok(())
}

// ── Deposit ─────────────────────────────────────────────────────────

async fn deposit(asset: &str, network: Option<&str>, json_output: bool) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();

    let resp = okx::get_deposit_address(&client, &api_key, &api_secret, &passphrase, asset).await?;

    let chain_filter = network.map(|n| okx::map_chain(asset, n));

    if json_output {
        let mut addresses = Vec::new();
        if let Some(data) = resp["data"].as_array() {
            for d in data {
                let chain = d["chain"].as_str().unwrap_or("");
                if let Some(ref filter) = chain_filter {
                    if chain != filter {
                        continue;
                    }
                }
                addresses.push(json!({
                    "address": d["addr"],
                    "chain": chain,
                    "tag": d["tag"],
                    "minDeposit": d["minDep"],
                }));
            }
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "deposit",
                "exchange": "okx",
                "asset": asset.to_uppercase(),
                "addresses": addresses,
            }))?
        );
        return Ok(());
    }

    println!();
    println!(
        "  {} {} → OKX",
        "Deposit".green().bold(),
        asset.to_uppercase().cyan()
    );
    println!();

    if let Some(data) = resp["data"].as_array() {
        for d in data {
            let chain = d["chain"].as_str().unwrap_or("?");
            if let Some(ref filter) = chain_filter {
                if chain != filter {
                    continue;
                }
            }
            let addr = d["addr"].as_str().unwrap_or("?");
            let tag = d["tag"].as_str().unwrap_or("");
            let min_dep = d["minDep"].as_str().unwrap_or("?");

            println!("  {} {}", "Chain:  ".dimmed(), chain.yellow());
            println!("  {} {}", "Address:".dimmed(), addr.green().bold());
            if !tag.is_empty() {
                println!("  {} {}", "Tag:    ".dimmed(), tag.yellow().bold());
            }
            println!(
                "  {} {} {}",
                "Min:    ".dimmed(),
                min_dep,
                asset.to_uppercase()
            );
            println!();
        }
    }

    Ok(())
}

// ── Withdraw ────────────────────────────────────────────────────────

async fn withdraw_cmd(
    asset: &str,
    amount: &str,
    to: Option<&str>,
    network: Option<&str>,
    fee: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();

    // Resolve destination address: use --to or fall back to config wallet
    let dest_addr = match to {
        Some(addr) => addr.to_string(),
        None => {
            let cfg = config::load_hl_config()?;
            cfg.address
        }
    };

    let chain = deposit_network(asset, network);

    // If fee not specified, look it up from OKX currency info
    let fee_str = match fee {
        Some(f) => f.to_string(),
        None => {
            let currencies = okx::get_currencies(
                &client,
                &api_key,
                &api_secret,
                &passphrase,
                Some(&asset.to_uppercase()),
            )
            .await?;

            // Find the matching chain entry
            let min_fee = currencies["data"]
                .as_array()
                .and_then(|arr| arr.iter().find(|c| c["chain"].as_str() == Some(&chain)))
                .and_then(|c| c["minFee"].as_str())
                .unwrap_or("0");

            min_fee.to_string()
        }
    };

    let resp = okx::withdraw(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        &asset.to_uppercase(),
        amount,
        "4", // on-chain withdrawal; OKX uses "4" for on-chain (internal is "3")
        &dest_addr,
        &chain,
        &fee_str,
    )
    .await?;

    let data = &resp["data"][0];
    let wd_id = data["wdId"].as_str().unwrap_or("unknown");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "withdraw",
                "exchange": "okx",
                "asset": asset.to_uppercase(),
                "amount": amount,
                "chain": chain,
                "toAddress": dest_addr,
                "fee": fee_str,
                "withdrawalId": wd_id,
                "response": data,
            }))?
        );
    } else {
        println!();
        println!("  {} Withdrawal submitted on OKX", "OK".green().bold());
        println!("  Asset:         {}", asset.to_uppercase().cyan());
        println!("  Amount:        {}", amount);
        println!("  Chain:         {}", chain.yellow());
        println!("  To:            {}", dest_addr);
        println!("  Fee:           {}", fee_str);
        println!("  Withdrawal ID: {}", wd_id);
        println!();
    }
    Ok(())
}

// ── Transfer ────────────────────────────────────────────────────────

async fn transfer_cmd(
    asset: &str,
    amount: &str,
    from: &str,
    to: &str,
    json_output: bool,
) -> Result<()> {
    let (api_key, api_secret, passphrase) = get_credentials()?;
    let client = reqwest::Client::new();

    let from_type = account_type(from)?;
    let to_type = account_type(to)?;

    let resp = okx::transfer(
        &client,
        &api_key,
        &api_secret,
        &passphrase,
        &asset.to_uppercase(),
        amount,
        from_type,
        to_type,
    )
    .await?;

    let data = &resp["data"][0];
    let trans_id = data["transId"].as_str().unwrap_or("unknown");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "transfer",
                "exchange": "okx",
                "asset": asset.to_uppercase(),
                "amount": amount,
                "from": from,
                "to": to,
                "transactionId": trans_id,
                "response": data,
            }))?
        );
    } else {
        println!();
        println!(
            "  {} Transferred {} {} from {} to {} on OKX",
            "OK".green().bold(),
            amount,
            asset.to_uppercase().cyan(),
            from,
            to
        );
        println!("  Transaction ID: {}", trans_id);
        println!();
    }
    Ok(())
}

// ── Funding rate ────────────────────────────────────────────────────

async fn funding_rate(symbol: &str, json_output: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let inst_id = okx::swap_inst_id(symbol);

    let resp = okx::get_funding_rate(&client, &inst_id).await?;

    let data = &resp["data"][0];
    let rate = data["fundingRate"].as_str().unwrap_or("?");
    let next_rate = data["nextFundingRate"].as_str().unwrap_or("?");
    let funding_time = data["fundingTime"].as_str().unwrap_or("?");

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "okx",
                "instId": inst_id,
                "fundingRate": rate,
                "nextFundingRate": next_rate,
                "fundingTime": funding_time,
            }))?
        );
    } else {
        let rate_f: f64 = rate.parse().unwrap_or(0.0);
        let rate_pct = format!("{:.4}%", rate_f * 100.0);
        let rate_colored = if rate_f >= 0.0 {
            rate_pct.green()
        } else {
            rate_pct.red()
        };

        println!();
        println!("  {} {} Funding Rate", inst_id.cyan().bold(), "OKX".green());
        println!();
        println!("  Current rate:  {}", rate_colored);
        println!("  Next rate:     {}", next_rate);
        println!("  Funding time:  {}", funding_time);
        println!();
    }
    Ok(())
}

// ── JSON mode ───────────────────────────────────────────────────────

fn default_levels() -> usize {
    5
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum JsonCommand {
    Buy {
        symbol: String,
        amount: f64,
        price: f64,
    },
    Sell {
        symbol: String,
        amount: f64,
        price: f64,
    },
    Orderbook {
        symbol: String,
        #[serde(default = "default_levels")]
        levels: usize,
    },
    Orders {
        symbol: Option<String>,
    },
    Cancel {
        inst_id: String,
        order_id: String,
    },
    Balance,
    Positions,
    PerpOrderbook {
        symbol: String,
        #[serde(default = "default_levels")]
        levels: usize,
    },
    PerpBuy {
        symbol: String,
        amount: f64,
        price: f64,
        #[serde(default)]
        close: bool,
    },
    PerpSell {
        symbol: String,
        amount: f64,
        price: f64,
        #[serde(default)]
        close: bool,
    },
    PerpLeverage {
        symbol: String,
        leverage: u32,
        #[serde(default)]
        cross: bool,
    },
    PerpFundingRate {
        symbol: String,
    },
    Deposit {
        asset: String,
        network: Option<String>,
    },
    Withdraw {
        asset: String,
        amount: f64,
        to: Option<String>,
        network: Option<String>,
        fee: Option<String>,
    },
    Quote {
        symbol: String,
    },
    Transfer {
        asset: String,
        amount: f64,
        from: String,
        to: String,
    },
}

async fn run_json(json_str: &str) -> Result<()> {
    let cmd: JsonCommand = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON command: {}", e))?;

    match cmd {
        JsonCommand::Buy {
            symbol,
            amount,
            price,
        } => spot_buy(&symbol, &fmt_num(amount), &fmt_num(price), true).await,
        JsonCommand::Sell {
            symbol,
            amount,
            price,
        } => spot_sell(&symbol, &fmt_num(amount), &fmt_num(price), true).await,
        JsonCommand::Orderbook { symbol, levels } => {
            let inst_id = okx::spot_inst_id(&symbol);
            orderbook(&inst_id, levels, true).await
        }
        JsonCommand::Orders { symbol } => {
            // Map symbol to instType if provided
            let inst_type = symbol.as_deref().map(|s| match s.to_uppercase().as_str() {
                "SPOT" => "SPOT",
                "SWAP" | "PERP" => "SWAP",
                "FUTURES" => "FUTURES",
                "OPTION" => "OPTION",
                _ => "SPOT",
            });
            orders(inst_type, true).await
        }
        JsonCommand::Cancel { inst_id, order_id } => cancel(&inst_id, &order_id, true).await,
        JsonCommand::Balance => balance(true).await,
        JsonCommand::Positions => positions(true).await,
        JsonCommand::PerpOrderbook { symbol, levels } => {
            let inst_id = okx::swap_inst_id(&symbol);
            orderbook(&inst_id, levels, true).await
        }
        JsonCommand::PerpBuy {
            symbol,
            amount,
            price,
            close,
        } => perp_buy(&symbol, &fmt_num(amount), &fmt_num(price), close, true).await,
        JsonCommand::PerpSell {
            symbol,
            amount,
            price,
            close,
        } => perp_sell(&symbol, &fmt_num(amount), &fmt_num(price), close, true).await,
        JsonCommand::PerpLeverage {
            symbol,
            leverage,
            cross,
        } => set_leverage(&symbol, leverage, cross, true).await,
        JsonCommand::PerpFundingRate { symbol } => funding_rate(&symbol, true).await,
        JsonCommand::Deposit { asset, network } => deposit(&asset, network.as_deref(), true).await,
        JsonCommand::Withdraw {
            asset,
            amount,
            to,
            network,
            fee,
        } => {
            let amount_str = fmt_num(amount);
            let (resolved_to, resolved_network) =
                resolve_withdraw_destination(to.as_deref(), network.as_deref());
            withdraw_cmd(
                &asset,
                &amount_str,
                resolved_to.as_deref(),
                resolved_network.as_deref(),
                fee.as_deref(),
                true,
            )
            .await
        }
        JsonCommand::Quote { symbol } => quote(&symbol, true).await,
        JsonCommand::Transfer {
            asset,
            amount,
            from,
            to,
        } => transfer_cmd(&asset, &fmt_num(amount), &from, &to, true).await,
    }
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = <Cli as clap::Parser>::parse();

    if let Some(ref json_str) = cli.json {
        let result = run_json(json_str).await;
        if let Err(e) = result {
            let err_json = serde_json::json!({"error": format!("{:#}", e)});
            println!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            std::process::exit(1);
        }
        return Ok(());
    }

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            <Cli as clap::Parser>::parse_from(["okx", "--help"]);
            unreachable!()
        }
    };
    let json_output = false;

    let result = match command {
        Commands::Buy {
            symbol,
            amount,
            price,
        } => spot_buy(&symbol, &amount, &price, json_output).await,
        Commands::Sell {
            symbol,
            amount,
            price,
        } => spot_sell(&symbol, &amount, &price, json_output).await,
        Commands::Orderbook { symbol, levels } => {
            let inst_id = okx::spot_inst_id(&symbol);
            orderbook(&inst_id, levels, json_output).await
        }
        Commands::Orders { symbol } => {
            let inst_type = symbol.as_deref().map(|s| match s.to_uppercase().as_str() {
                "SPOT" => "SPOT",
                "SWAP" | "PERP" => "SWAP",
                "FUTURES" => "FUTURES",
                "OPTION" => "OPTION",
                _ => "SPOT",
            });
            orders(inst_type, json_output).await
        }
        Commands::Cancel { inst_id, order_id } => cancel(&inst_id, &order_id, json_output).await,
        Commands::Balance => balance(json_output).await,
        Commands::Positions => positions(json_output).await,
        Commands::Perp(cmd) => match cmd {
            PerpCmd::Orderbook { symbol, levels } => {
                let inst_id = okx::swap_inst_id(&symbol);
                orderbook(&inst_id, levels, json_output).await
            }
            PerpCmd::Buy {
                symbol,
                amount,
                price,
                close,
            } => perp_buy(&symbol, &amount, &price, close, json_output).await,
            PerpCmd::Sell {
                symbol,
                amount,
                price,
                close,
            } => perp_sell(&symbol, &amount, &price, close, json_output).await,
            PerpCmd::Leverage {
                symbol,
                leverage,
                cross,
            } => set_leverage(&symbol, leverage, cross, json_output).await,
            PerpCmd::FundingRate { symbol } => funding_rate(&symbol, json_output).await,
        },
        Commands::Deposit { asset, network } => {
            deposit(&asset, network.as_deref(), json_output).await
        }
        Commands::Withdraw {
            asset,
            amount,
            to,
            network,
            fee,
        } => {
            let (resolved_to, resolved_network) =
                resolve_withdraw_destination(to.as_deref(), network.as_deref());
            withdraw_cmd(
                &asset,
                &amount,
                resolved_to.as_deref(),
                resolved_network.as_deref(),
                fee.as_deref(),
                json_output,
            )
            .await
        }
        Commands::Quote { symbol } => quote(&symbol, json_output).await,
        Commands::Transfer {
            asset,
            amount,
            from,
            to,
        } => transfer_cmd(&asset, &amount, &from, &to, json_output).await,
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}
