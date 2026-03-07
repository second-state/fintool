/// JSON mode: parse a JSON command string and dispatch to the appropriate handler.
///
/// Usage: fintool --json '{"command":"quote","symbol":"BTC"}'
///
/// In JSON mode, all output is JSON (json_output = true).
use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{commands, config, signing};

/// Format f64 to string with 8 decimal places, then strip trailing zeros.
/// Matches Hyperliquid SDK's float_to_string_for_hashing convention.
fn fmt_num(val: f64) -> String {
    let s = format!("{:.8}", val);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn default_exchange() -> String {
    "auto".to_string()
}

fn default_limit() -> usize {
    10
}

fn default_predict_limit() -> i32 {
    10
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum JsonCommand {
    Init,
    Address,
    Quote {
        symbol: String,
    },
    News {
        symbol: String,
    },
    OrderBuy {
        symbol: String,
        amount: f64,
        price: f64,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    OrderSell {
        symbol: String,
        amount: f64,
        price: f64,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    Orders {
        symbol: Option<String>,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    Cancel {
        order_id: String,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    Balance {
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    Positions {
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    PerpQuote {
        symbol: String,
    },
    PerpBuy {
        symbol: String,
        amount: f64,
        price: f64,
        #[serde(default)]
        close: bool,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    PerpSell {
        symbol: String,
        amount: f64,
        price: f64,
        #[serde(default)]
        close: bool,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    PerpLeverage {
        symbol: String,
        leverage: u32,
        #[serde(default)]
        cross: bool,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    PerpSetMode {
        mode: String,
    },
    OptionsBuy {
        symbol: String,
        option_type: String,
        strike: String,
        expiry: String,
        size: String,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    OptionsSell {
        symbol: String,
        option_type: String,
        strike: String,
        expiry: String,
        size: String,
        #[serde(default = "default_exchange")]
        exchange: String,
    },
    Deposit {
        asset: String,
        amount: Option<f64>,
        from: Option<String>,
        #[serde(default = "default_exchange")]
        exchange: String,
        #[serde(default)]
        dry_run: bool,
    },
    Withdraw {
        asset: String,
        amount: f64,
        to: Option<String>,
        network: Option<String>,
        #[serde(default)]
        dry_run: bool,
    },
    Transfer {
        asset: String,
        amount: f64,
        from: String,
        to: String,
    },
    BridgeStatus,
    ReportAnnual {
        symbol: String,
        output: Option<String>,
    },
    ReportQuarterly {
        symbol: String,
        output: Option<String>,
    },
    ReportList {
        symbol: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },
    ReportGet {
        symbol: String,
        accession: String,
        output: Option<String>,
    },
    PredictList {
        query: Option<String>,
        #[serde(default = "default_predict_limit")]
        limit: i32,
        active: Option<bool>,
        sort: Option<String>,
    },
    PredictQuote {
        market: String,
    },
    PredictBuy {
        market: String,
        outcome: String,
        amount: f64,
        price: f64,
    },
    PredictSell {
        market: String,
        outcome: String,
        amount: f64,
        price: f64,
    },
    PredictPositions,
}

pub async fn run(json_str: &str) -> Result<()> {
    let cmd: JsonCommand = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON command: {}", e))?;

    match cmd {
        JsonCommand::Init => {
            let (path, created) = config::init_config()?;
            let msg = if created {
                format!("Config file created at: {}", path.display())
            } else {
                format!("Config file already exists at: {}", path.display())
            };
            println!("{}", serde_json::json!({"action": "init", "message": msg}));
            Ok(())
        }
        JsonCommand::Address => {
            let address = signing::get_wallet_address().context("No wallet configured")?;
            println!("{}", serde_json::json!({"address": address}));
            Ok(())
        }
        JsonCommand::Quote { symbol } => commands::quote::run_spot(&symbol, true).await,
        JsonCommand::News { symbol } => commands::news::run(&symbol, true).await,
        JsonCommand::OrderBuy {
            symbol,
            amount,
            price,
            exchange,
        } => {
            let amount = fmt_num(amount);
            let price = fmt_num(price);
            commands::order::buy(&symbol, &amount, &price, &exchange, true).await
        }
        JsonCommand::OrderSell {
            symbol,
            amount,
            price,
            exchange,
        } => {
            let amount = fmt_num(amount);
            let price = fmt_num(price);
            commands::order::sell(&symbol, &amount, &price, &exchange, true).await
        }
        JsonCommand::Orders { symbol, exchange } => {
            commands::orders::run(symbol.as_deref(), &exchange, true).await
        }
        JsonCommand::Cancel { order_id, exchange } => {
            commands::cancel::run(&order_id, &exchange, true).await
        }
        JsonCommand::Balance { exchange } => commands::balance::run(&exchange, true).await,
        JsonCommand::Positions { exchange } => commands::positions::run(&exchange, true).await,
        JsonCommand::PerpQuote { symbol } => commands::quote::run_perp(&symbol, true).await,
        JsonCommand::PerpBuy {
            symbol,
            amount,
            price,
            close,
            exchange,
        } => {
            let amount = fmt_num(amount);
            let price = fmt_num(price);
            commands::perp::buy(&symbol, &amount, &price, close, &exchange, true).await
        }
        JsonCommand::PerpSell {
            symbol,
            amount,
            price,
            close,
            exchange,
        } => {
            let amount = fmt_num(amount);
            let price = fmt_num(price);
            commands::perp::sell(&symbol, &amount, &price, close, &exchange, true).await
        }
        JsonCommand::PerpLeverage {
            symbol,
            leverage,
            cross,
            exchange,
        } => commands::perp::set_leverage(&symbol, leverage, cross, &exchange, true).await,
        JsonCommand::PerpSetMode { mode } => commands::perp::set_mode(&mode, true).await,
        JsonCommand::OptionsBuy {
            symbol,
            option_type,
            strike,
            expiry,
            size,
            exchange,
        } => {
            commands::options::buy(
                &symbol,
                &option_type,
                &strike,
                &expiry,
                &size,
                &exchange,
                true,
            )
            .await
        }
        JsonCommand::OptionsSell {
            symbol,
            option_type,
            strike,
            expiry,
            size,
            exchange,
        } => {
            commands::options::sell(
                &symbol,
                &option_type,
                &strike,
                &expiry,
                &size,
                &exchange,
                true,
            )
            .await
        }
        JsonCommand::Deposit {
            asset,
            amount,
            from,
            exchange,
            dry_run,
        } => {
            let amount = amount.map(fmt_num);
            commands::deposit::run(
                &asset,
                amount.as_deref(),
                from.as_deref(),
                &exchange,
                dry_run,
                true,
            )
            .await
        }
        JsonCommand::Withdraw {
            asset,
            amount,
            to,
            network,
            dry_run,
        } => {
            let amount = fmt_num(amount);
            let (resolved_to, resolved_network) =
                crate::resolve_withdraw_destination(to.as_deref(), network.as_deref());
            commands::withdraw::run(
                &amount,
                &asset,
                resolved_to.as_deref(),
                resolved_network.as_deref(),
                "auto",
                dry_run,
                true,
            )
            .await
        }
        JsonCommand::Transfer {
            asset,
            amount,
            from,
            to,
        } => {
            let amount = fmt_num(amount);
            commands::transfer::run(&asset, &amount, &from, &to, true).await
        }
        JsonCommand::BridgeStatus => commands::bridge_status::run(true).await,
        JsonCommand::ReportAnnual { symbol, output } => {
            commands::report::annual(&symbol, output.as_deref(), true).await
        }
        JsonCommand::ReportQuarterly { symbol, output } => {
            commands::report::quarterly(&symbol, output.as_deref(), true).await
        }
        JsonCommand::ReportList { symbol, limit } => {
            commands::report::list(&symbol, limit, true).await
        }
        JsonCommand::ReportGet {
            symbol,
            accession,
            output,
        } => commands::report::get(&symbol, &accession, output.as_deref(), true).await,
        JsonCommand::PredictList {
            query,
            limit,
            active,
            sort,
        } => commands::predict::list(query.as_deref(), limit, active, sort.as_deref(), true).await,
        JsonCommand::PredictQuote { market } => commands::predict::quote(&market, true).await,
        JsonCommand::PredictBuy {
            market,
            outcome,
            amount,
            price,
        } => {
            let amount = fmt_num(amount);
            let price = fmt_num(price);
            commands::predict::buy(&market, &outcome, &amount, &price, true).await
        }
        JsonCommand::PredictSell {
            market,
            outcome,
            amount,
            price,
        } => {
            let amount = fmt_num(amount);
            let price = fmt_num(price);
            commands::predict::sell(&market, &outcome, &amount, &price, true).await
        }
        JsonCommand::PredictPositions => commands::predict::positions(true).await,
    }
}
