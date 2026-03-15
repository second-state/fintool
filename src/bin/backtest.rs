use anyhow::Result;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::Deserialize;
use serde_json::json;

use fintool_lib::backtest::{self, Portfolio, TradeSide, TradeType};
use fintool_lib::commands;

#[derive(Parser)]
#[command(
    name = "backtest",
    about = "Backtesting CLI — simulate trades at historical dates with forward PnL analysis"
)]
struct Cli {
    /// Historical date to simulate (YYYY-MM-DD format)
    #[arg(long)]
    at: String,

    #[command(subcommand)]
    command: Option<Commands>,

    /// JSON mode: pass a JSON command string for programmatic use (always outputs JSON).
    #[arg(long)]
    json: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Get historical price at the backtest date
    Quote { symbol: String },

    /// Show news (stub — historical news not available)
    News { symbol: String },

    /// SEC filings on or before the backtest date
    #[command(subcommand)]
    Report(ReportCmd),

    /// Simulated spot buy with forward PnL
    Buy {
        symbol: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        price: Option<f64>,
    },

    /// Simulated spot sell with forward PnL
    Sell {
        symbol: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        price: Option<f64>,
    },

    /// Perpetual futures simulation
    #[command(subcommand)]
    Perp(PerpCmd),

    /// Show simulated portfolio balance
    Balance,

    /// Show simulated open positions
    Positions,

    /// Reset simulated portfolio (clear all trades and positions)
    Reset,
}

#[derive(Subcommand)]
enum ReportCmd {
    /// Latest 10-K annual filing on or before the backtest date
    Annual {
        symbol: String,
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Latest 10-Q quarterly filing on or before the backtest date
    Quarterly {
        symbol: String,
        #[arg(long, short)]
        output: Option<String>,
    },
    /// List recent filings on or before the backtest date
    List {
        symbol: String,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Fetch a specific filing by accession number
    Get {
        symbol: String,
        accession: String,
        #[arg(long, short)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum PerpCmd {
    /// Simulated perp long with forward PnL
    Buy {
        symbol: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        price: Option<f64>,
        #[arg(long)]
        close: bool,
    },
    /// Simulated perp short with forward PnL
    Sell {
        symbol: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        price: Option<f64>,
        #[arg(long)]
        close: bool,
    },
    /// Set leverage for PnL calculation
    Leverage {
        symbol: String,
        #[arg(long)]
        leverage: u32,
    },
}

// ── JSON mode ───────────────────────────────────────────────────────────

fn default_limit() -> usize {
    10
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum JsonCommand {
    Quote {
        symbol: String,
    },
    News {
        symbol: String,
    },
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
    Buy {
        symbol: String,
        amount: f64,
        price: Option<f64>,
    },
    Sell {
        symbol: String,
        amount: f64,
        price: Option<f64>,
    },
    PerpBuy {
        symbol: String,
        amount: f64,
        price: Option<f64>,
    },
    PerpSell {
        symbol: String,
        amount: f64,
        price: Option<f64>,
    },
    PerpLeverage {
        symbol: String,
        leverage: u32,
    },
    Balance,
    Positions,
    Reset,
}

// ── Command handlers ────────────────────────────────────────────────────

async fn cmd_quote(symbol: &str, date: NaiveDate, json_output: bool) -> Result<()> {
    let price = backtest::fetch_price_at_date(symbol, date).await?;
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "symbol": symbol.to_uppercase(),
                "date": date.to_string(),
                "price": format!("{:.2}", price),
            }))?
        );
    } else {
        println!();
        println!(
            "  {} historical price on {}",
            symbol.to_uppercase().bold().cyan(),
            date
        );
        println!("  Price: ${:.2}", price);
        println!();
    }
    Ok(())
}

async fn cmd_news(symbol: &str, _date: NaiveDate, json_output: bool) -> Result<()> {
    let msg = "Historical news not available for backtesting. Use `fintool news` for current news.";
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "symbol": symbol.to_uppercase(),
                "message": msg,
            }))?
        );
    } else {
        println!("\n  {}\n", msg);
    }
    Ok(())
}

async fn cmd_report_list(
    symbol: &str,
    limit: usize,
    date: NaiveDate,
    json_output: bool,
) -> Result<()> {
    let date_str = date.to_string();
    let (cik, company) = commands::report::resolve_cik(symbol).await?;
    let filings = commands::report::get_filings(cik, None, limit, Some(&date_str)).await?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&filings)?);
    } else {
        println!(
            "\n  {} recent filings for {} ({}) on or before {}:\n",
            limit,
            company.bold(),
            symbol.to_uppercase(),
            date,
        );
        println!(
            "  {:<8} {:<12} {:<12} Accession Number",
            "Form", "Filed", "Period"
        );
        println!("  {}", "-".repeat(66));
        for f in &filings {
            println!(
                "  {:<8} {:<12} {:<12} {}",
                f.form, f.filing_date, f.report_date, f.accession_number
            );
        }
        println!();
    }
    Ok(())
}

async fn cmd_report_annual(
    symbol: &str,
    output: Option<&str>,
    date: NaiveDate,
    json_output: bool,
) -> Result<()> {
    let date_str = date.to_string();
    let (cik, _company) = commands::report::resolve_cik(symbol).await?;
    let filings = commands::report::get_filings(cik, Some("10-K"), 1, Some(&date_str)).await?;
    let filing = filings
        .first()
        .ok_or_else(|| anyhow::anyhow!("No 10-K filing found for {} before {}", symbol, date))?;
    commands::report::get(symbol, &filing.accession_number, output, json_output).await
}

async fn cmd_report_quarterly(
    symbol: &str,
    output: Option<&str>,
    date: NaiveDate,
    json_output: bool,
) -> Result<()> {
    let date_str = date.to_string();
    let (cik, _company) = commands::report::resolve_cik(symbol).await?;
    let filings = commands::report::get_filings(cik, Some("10-Q"), 1, Some(&date_str)).await?;
    let filing = filings
        .first()
        .ok_or_else(|| anyhow::anyhow!("No 10-Q filing found for {} before {}", symbol, date))?;
    commands::report::get(symbol, &filing.accession_number, output, json_output).await
}

#[allow(clippy::too_many_arguments)]
async fn cmd_trade(
    symbol: &str,
    amount: f64,
    price: Option<f64>,
    side: TradeSide,
    trade_type: TradeType,
    date: NaiveDate,
    portfolio: &mut Portfolio,
    json_output: bool,
) -> Result<()> {
    let price = match price {
        Some(p) => p,
        None => backtest::fetch_price_at_date(symbol, date).await?,
    };

    let leverage = if trade_type == TradeType::Perp {
        portfolio.get_leverage(symbol)
    } else {
        1
    };

    let trade = portfolio.add_trade(symbol, side, amount, price, date, trade_type);
    portfolio.save()?;

    let pnl_prices = backtest::fetch_pnl_prices(symbol, date).await?;

    if json_output {
        let mut output = backtest::build_pnl_json(&trade, &pnl_prices, leverage);
        output["portfolio"] = backtest::build_portfolio_json(portfolio);
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        backtest::print_pnl_table(&trade, &pnl_prices, leverage)?;
        backtest::print_portfolio_summary(portfolio);
    }
    Ok(())
}

// ── JSON dispatch ───────────────────────────────────────────────────────

async fn run_json(json_str: &str, date: NaiveDate, portfolio: &mut Portfolio) -> Result<()> {
    let cmd: JsonCommand = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON command: {}", e))?;

    match cmd {
        JsonCommand::Quote { symbol } => cmd_quote(&symbol, date, true).await,
        JsonCommand::News { symbol } => cmd_news(&symbol, date, true).await,
        JsonCommand::ReportList { symbol, limit } => {
            cmd_report_list(&symbol, limit, date, true).await
        }
        JsonCommand::ReportAnnual { symbol, output } => {
            cmd_report_annual(&symbol, output.as_deref(), date, true).await
        }
        JsonCommand::ReportQuarterly { symbol, output } => {
            cmd_report_quarterly(&symbol, output.as_deref(), date, true).await
        }
        JsonCommand::ReportGet {
            symbol,
            accession,
            output,
        } => commands::report::get(&symbol, &accession, output.as_deref(), true).await,
        JsonCommand::Buy {
            symbol,
            amount,
            price,
        } => {
            cmd_trade(
                &symbol,
                amount,
                price,
                TradeSide::Buy,
                TradeType::Spot,
                date,
                portfolio,
                true,
            )
            .await
        }
        JsonCommand::Sell {
            symbol,
            amount,
            price,
        } => {
            cmd_trade(
                &symbol,
                amount,
                price,
                TradeSide::Sell,
                TradeType::Spot,
                date,
                portfolio,
                true,
            )
            .await
        }
        JsonCommand::PerpBuy {
            symbol,
            amount,
            price,
            ..
        } => {
            cmd_trade(
                &symbol,
                amount,
                price,
                TradeSide::Buy,
                TradeType::Perp,
                date,
                portfolio,
                true,
            )
            .await
        }
        JsonCommand::PerpSell {
            symbol,
            amount,
            price,
            ..
        } => {
            cmd_trade(
                &symbol,
                amount,
                price,
                TradeSide::Sell,
                TradeType::Perp,
                date,
                portfolio,
                true,
            )
            .await
        }
        JsonCommand::PerpLeverage { symbol, leverage } => {
            portfolio.set_leverage(&symbol, leverage);
            portfolio.save()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "symbol": symbol.to_uppercase(),
                    "leverage": leverage,
                }))?
            );
            Ok(())
        }
        JsonCommand::Balance => {
            let cash = portfolio.cash_balance();
            let positions = portfolio.positions();
            let pos_json: Vec<serde_json::Value> = positions
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
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "cashBalance": format!("{:.2}", cash),
                    "positions": pos_json,
                    "totalTrades": portfolio.trade_count(),
                    "leverageSettings": &portfolio.leverage_settings,
                }))?
            );
            Ok(())
        }
        JsonCommand::Positions => {
            let positions = portfolio.positions();
            let pos_json: Vec<serde_json::Value> = positions
                .iter()
                .map(|p| {
                    json!({
                        "symbol": p.symbol,
                        "type": p.trade_type,
                        "side": p.side,
                        "quantity": p.net_quantity,
                        "avgEntryPrice": format!("{:.2}", p.avg_entry_price),
                        "totalCost": format!("{:.2}", p.total_cost),
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "positions": pos_json,
                }))?
            );
            Ok(())
        }
        JsonCommand::Reset => {
            portfolio.reset();
            portfolio.save()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status": "ok",
                    "message": "Portfolio reset. All trades and positions cleared.",
                }))?
            );
            Ok(())
        }
    }
}

// ── Main ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Parse the --at date
    let date = NaiveDate::parse_from_str(&cli.at, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid date '{}': {}. Use YYYY-MM-DD format.", cli.at, e))?;

    // Validate date is not in the future
    let today = chrono::Utc::now().date_naive();
    if date > today {
        anyhow::bail!("Backtest date {} is in the future. Use a past date.", date);
    }

    let mut portfolio = Portfolio::load().unwrap_or_else(|e| {
        eprintln!(
            "Warning: could not load portfolio state: {:#}. Starting fresh.",
            e
        );
        Portfolio::new()
    });

    // JSON mode
    if let Some(ref json_str) = cli.json {
        let result = run_json(json_str, date, &mut portfolio).await;
        if let Err(e) = result {
            let err_json = json!({"error": format!("{:#}", e)});
            println!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            std::process::exit(1);
        }
        return Ok(());
    }

    // CLI mode
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            Cli::parse_from(["backtest", "--at", &cli.at, "--help"]);
            unreachable!()
        }
    };
    let json_output = false;

    let result = match command {
        Commands::Quote { symbol } => cmd_quote(&symbol, date, json_output).await,
        Commands::News { symbol } => cmd_news(&symbol, date, json_output).await,
        Commands::Report(cmd) => match cmd {
            ReportCmd::Annual { symbol, output } => {
                cmd_report_annual(&symbol, output.as_deref(), date, json_output).await
            }
            ReportCmd::Quarterly { symbol, output } => {
                cmd_report_quarterly(&symbol, output.as_deref(), date, json_output).await
            }
            ReportCmd::List { symbol, limit } => {
                cmd_report_list(&symbol, limit, date, json_output).await
            }
            ReportCmd::Get {
                symbol,
                accession,
                output,
            } => commands::report::get(&symbol, &accession, output.as_deref(), json_output).await,
        },
        Commands::Buy {
            symbol,
            amount,
            price,
        } => {
            cmd_trade(
                &symbol,
                amount,
                price,
                TradeSide::Buy,
                TradeType::Spot,
                date,
                &mut portfolio,
                json_output,
            )
            .await
        }
        Commands::Sell {
            symbol,
            amount,
            price,
        } => {
            cmd_trade(
                &symbol,
                amount,
                price,
                TradeSide::Sell,
                TradeType::Spot,
                date,
                &mut portfolio,
                json_output,
            )
            .await
        }
        Commands::Perp(cmd) => match cmd {
            PerpCmd::Buy {
                symbol,
                amount,
                price,
                ..
            } => {
                cmd_trade(
                    &symbol,
                    amount,
                    price,
                    TradeSide::Buy,
                    TradeType::Perp,
                    date,
                    &mut portfolio,
                    json_output,
                )
                .await
            }
            PerpCmd::Sell {
                symbol,
                amount,
                price,
                ..
            } => {
                cmd_trade(
                    &symbol,
                    amount,
                    price,
                    TradeSide::Sell,
                    TradeType::Perp,
                    date,
                    &mut portfolio,
                    json_output,
                )
                .await
            }
            PerpCmd::Leverage { symbol, leverage } => {
                portfolio.set_leverage(&symbol, leverage);
                portfolio.save()?;
                println!(
                    "\n  Leverage for {} set to {}x\n",
                    symbol.to_uppercase().bold(),
                    leverage
                );
                Ok(())
            }
        },
        Commands::Balance => {
            let cash = portfolio.cash_balance();
            let positions = portfolio.positions();
            println!("\n  {} Simulated portfolio", "[BACKTEST]".dimmed());
            let cash_str = format!("${:.2}", cash);
            let colored_cash = if cash >= 0.0 {
                cash_str.green().to_string()
            } else {
                cash_str.red().to_string()
            };
            println!("  Cash balance: {}", colored_cash);
            println!("  Total trades: {}", portfolio.trade_count());
            if !positions.is_empty() {
                println!("  Open positions: {}", positions.len());
            }
            println!();
            Ok(())
        }
        Commands::Positions => {
            let positions = portfolio.positions();
            if positions.is_empty() {
                println!("\n  {} No open positions.\n", "[BACKTEST]".dimmed());
            } else {
                println!("\n  {} Open positions:\n", "[BACKTEST]".dimmed());
                println!(
                    "  {:<10} {:<6} {:<8} {:>12} {:>14}",
                    "Symbol", "Type", "Side", "Quantity", "Avg Entry"
                );
                println!("  {}", "-".repeat(54));
                for p in &positions {
                    let type_str = match p.trade_type {
                        TradeType::Spot => "spot",
                        TradeType::Perp => "perp",
                    };
                    println!(
                        "  {:<10} {:<6} {:<8} {:>12.4} {:>14.2}",
                        p.symbol,
                        type_str,
                        p.side,
                        p.net_quantity.abs(),
                        p.avg_entry_price
                    );
                }
                println!();
            }
            Ok(())
        }
        Commands::Reset => {
            portfolio.reset();
            portfolio.save()?;
            println!(
                "\n  {} Portfolio reset. All trades and positions cleared.\n",
                "[BACKTEST]".dimmed()
            );
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}
