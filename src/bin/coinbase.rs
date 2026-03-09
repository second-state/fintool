use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::Deserialize;

use fintool_lib::{commands, format::fmt_num, resolve_withdraw_destination};

const EXCHANGE: &str = "coinbase";

#[derive(Parser)]
#[command(name = "coinbase", about = "Coinbase trading CLI — spot trading")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// JSON mode: pass a JSON command string for programmatic use (always outputs JSON).
    /// Example: coinbase --json '{"command":"buy","symbol":"ETH","amount":0.1,"price":2000}'
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

    /// Show L2 orderbook / market depth for a spot pair
    Orderbook {
        symbol: String,
        /// Number of price levels per side (default: 5)
        #[arg(long, default_value = "5")]
        levels: usize,
    },

    /// List open orders
    Orders { symbol: Option<String> },

    /// Cancel an order
    Cancel { order_id: String },

    /// Show account balances
    Balance,

    /// Deposit to Coinbase
    Deposit {
        /// Asset: ETH, BTC, USDC, etc.
        asset: String,
        /// Amount (if applicable)
        #[arg(long)]
        amount: Option<String>,
        /// Source chain
        #[arg(long)]
        from: Option<String>,
        /// Show quote only, don't execute
        #[arg(long)]
        dry_run: bool,
    },

    /// Withdraw from Coinbase
    Withdraw {
        /// Asset: ETH, BTC, USDC, etc.
        asset: String,
        /// Amount to withdraw
        #[arg(long)]
        amount: String,
        /// Destination: chain name or address
        #[arg(long)]
        to: Option<String>,
        /// Network (e.g. ethereum, base, arbitrum, solana)
        #[arg(long)]
        network: Option<String>,
        /// Show quote only, don't execute
        #[arg(long)]
        dry_run: bool,
    },
}

// --- JSON mode ---

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
        order_id: String,
    },
    Balance,
    Deposit {
        asset: String,
        amount: Option<f64>,
        from: Option<String>,
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
}

async fn run_json(json_str: &str) -> Result<()> {
    let cmd: JsonCommand = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON command: {}", e))?;

    match cmd {
        JsonCommand::Buy {
            symbol,
            amount,
            price,
        } => commands::order::buy(&symbol, &fmt_num(amount), &fmt_num(price), EXCHANGE, true).await,
        JsonCommand::Sell {
            symbol,
            amount,
            price,
        } => {
            commands::order::sell(&symbol, &fmt_num(amount), &fmt_num(price), EXCHANGE, true).await
        }
        JsonCommand::Orderbook { symbol, levels } => {
            commands::orderbook::run_spot(&symbol, levels, EXCHANGE, true).await
        }
        JsonCommand::Orders { symbol } => {
            commands::orders::run(symbol.as_deref(), EXCHANGE, true).await
        }
        JsonCommand::Cancel { order_id } => commands::cancel::run(&order_id, EXCHANGE, true).await,
        JsonCommand::Balance => commands::balance::run(EXCHANGE, true).await,
        JsonCommand::Deposit {
            asset,
            amount,
            from,
            dry_run,
        } => {
            let amount = amount.map(fmt_num);
            commands::deposit::run(
                &asset,
                amount.as_deref(),
                from.as_deref(),
                EXCHANGE,
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
                resolve_withdraw_destination(to.as_deref(), network.as_deref());
            commands::withdraw::run(
                &amount,
                &asset,
                resolved_to.as_deref(),
                resolved_network.as_deref(),
                EXCHANGE,
                dry_run,
                true,
            )
            .await
        }
    }
}

// --- Main ---

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
            <Cli as clap::Parser>::parse_from(["coinbase", "--help"]);
            unreachable!()
        }
    };
    let json_output = false;

    let result = match command {
        Commands::Buy {
            symbol,
            amount,
            price,
        } => commands::order::buy(&symbol, &amount, &price, EXCHANGE, json_output).await,
        Commands::Sell {
            symbol,
            amount,
            price,
        } => commands::order::sell(&symbol, &amount, &price, EXCHANGE, json_output).await,
        Commands::Orderbook { symbol, levels } => {
            commands::orderbook::run_spot(&symbol, levels, EXCHANGE, json_output).await
        }
        Commands::Orders { symbol } => {
            commands::orders::run(symbol.as_deref(), EXCHANGE, json_output).await
        }
        Commands::Cancel { order_id } => {
            commands::cancel::run(&order_id, EXCHANGE, json_output).await
        }
        Commands::Balance => commands::balance::run(EXCHANGE, json_output).await,
        Commands::Deposit {
            asset,
            amount,
            from,
            dry_run,
        } => {
            commands::deposit::run(
                &asset,
                amount.as_deref(),
                from.as_deref(),
                EXCHANGE,
                dry_run,
                json_output,
            )
            .await
        }
        Commands::Withdraw {
            asset,
            amount,
            to,
            network,
            dry_run,
        } => {
            let (resolved_to, resolved_network) =
                resolve_withdraw_destination(to.as_deref(), network.as_deref());
            commands::withdraw::run(
                &amount,
                &asset,
                resolved_to.as_deref(),
                resolved_network.as_deref(),
                EXCHANGE,
                dry_run,
                json_output,
            )
            .await
        }
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}
