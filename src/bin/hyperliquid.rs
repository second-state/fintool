use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::Deserialize;

use fintool_lib::{commands, format::fmt_num, resolve_withdraw_destination, signing};

const EXCHANGE: &str = "hyperliquid";

#[derive(Parser)]
#[command(
    name = "hyperliquid",
    about = "Hyperliquid trading CLI — spot, perpetual futures, and HIP-3 dex"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// JSON mode: pass a JSON command string for programmatic use (always outputs JSON).
    /// Example: hyperliquid --json '{"command":"buy","symbol":"ETH","amount":0.1,"price":2000}'
    #[arg(long)]
    json: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the configured wallet address
    Address,

    /// Get perpetual futures price quote (same as `perp quote`)
    Quote { symbol: String },

    /// Place a spot limit buy order
    Buy {
        symbol: String,
        /// Amount of the asset to buy (in symbol units, e.g. 1.0 HYPE)
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

    /// List open orders (spot and perp)
    Orders { symbol: Option<String> },

    /// Cancel an order
    Cancel { order_id: String },

    /// Show account balances
    Balance,

    /// Show open positions
    Positions,

    /// Options trading
    #[command(subcommand)]
    Options(OptionsCmd),

    /// Deposit to Hyperliquid: bridge ETH/USDC, or show deposit address for BTC/SOL
    Deposit {
        /// Asset: ETH, BTC, SOL, USDC, etc.
        asset: String,
        /// Amount to deposit (e.g. 0.01)
        #[arg(long)]
        amount: String,
        /// Source chain for USDC: ethereum or base
        #[arg(long)]
        from: Option<String>,
        /// Show quote only, don't execute transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Withdraw from Hyperliquid to external address
    Withdraw {
        /// Asset: ETH, BTC, SOL, USDC, etc.
        asset: String,
        /// Amount to withdraw (e.g. 10)
        #[arg(long)]
        amount: String,
        /// Destination: chain name (base, ethereum) or address (0x...)
        #[arg(long)]
        to: Option<String>,
        /// Network (e.g. ethereum, base, arbitrum)
        #[arg(long)]
        network: Option<String>,
        /// Show quote only, don't execute
        #[arg(long)]
        dry_run: bool,
    },

    /// Transfer assets between perp, spot, and HIP-3 dex accounts
    Transfer {
        /// Asset to transfer (e.g. USDC, USDT0)
        asset: String,
        /// Amount to transfer
        #[arg(long)]
        amount: String,
        /// Source: spot, perp, or a HIP-3 dex name (e.g. cash)
        #[arg(long)]
        from: String,
        /// Destination: spot, perp, or a HIP-3 dex name (e.g. cash)
        #[arg(long)]
        to: String,
    },

    /// Show bridge operation status (deposits/withdrawals via Unit)
    BridgeStatus,
}

#[derive(Subcommand)]
enum PerpCmd {
    /// Get perpetual futures price quote
    Quote { symbol: String },
    /// Show L2 orderbook / market depth for a perpetual
    Orderbook {
        symbol: String,
        /// Number of price levels per side (default: 5)
        #[arg(long, default_value = "5")]
        levels: usize,
    },
    /// Place a perp limit buy (long) order
    Buy {
        symbol: String,
        /// Size in asset units (e.g. 0.1 ETH)
        #[arg(long)]
        amount: String,
        /// Limit price
        #[arg(long)]
        price: String,
        /// Close position only (reduce-only, won't open a new long)
        #[arg(long)]
        close: bool,
    },
    /// Place a perp limit sell (short) order
    Sell {
        symbol: String,
        /// Size in asset units (e.g. 0.006 ETH)
        #[arg(long)]
        amount: String,
        /// Limit price
        #[arg(long)]
        price: String,
        /// Close position only (reduce-only, won't open a new short)
        #[arg(long)]
        close: bool,
    },
    /// Set leverage for a perp asset
    Leverage {
        symbol: String,
        /// Leverage multiplier (e.g. 5, 10, 20)
        #[arg(long)]
        leverage: u32,
        /// Use cross margin instead of isolated
        #[arg(long)]
        cross: bool,
    },
    /// Set account mode: unified (share margin across all dexes), standard, or disabled
    SetMode {
        /// Mode: "unified", "standard", or "disabled"
        mode: String,
    },
}

#[derive(Subcommand)]
enum OptionsCmd {
    /// Buy an option
    Buy {
        symbol: String,
        option_type: String,
        strike: String,
        expiry: String,
        size: String,
    },
    /// Sell an option
    Sell {
        symbol: String,
        option_type: String,
        strike: String,
        expiry: String,
        size: String,
    },
}

// --- JSON mode ---

fn default_levels() -> usize {
    5
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum JsonCommand {
    Address,
    Quote {
        symbol: String,
    },
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
    Positions,
    PerpQuote {
        symbol: String,
    },
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
    PerpSetMode {
        mode: String,
    },
    OptionsBuy {
        symbol: String,
        option_type: String,
        strike: String,
        expiry: String,
        size: String,
    },
    OptionsSell {
        symbol: String,
        option_type: String,
        strike: String,
        expiry: String,
        size: String,
    },
    Deposit {
        asset: String,
        amount: f64,
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
    Transfer {
        asset: String,
        amount: f64,
        from: String,
        to: String,
    },
    BridgeStatus,
}

async fn run_json(json_str: &str) -> Result<()> {
    let cmd: JsonCommand = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON command: {}", e))?;

    match cmd {
        JsonCommand::Address => {
            let address = signing::get_wallet_address().context("No wallet configured")?;
            println!("{}", serde_json::json!({"address": address}));
            Ok(())
        }
        JsonCommand::Quote { symbol } | JsonCommand::PerpQuote { symbol } => {
            commands::quote::run_perp(&symbol, true).await
        }
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
        JsonCommand::Positions => commands::positions::run(EXCHANGE, true).await,
        JsonCommand::PerpOrderbook { symbol, levels } => {
            commands::orderbook::run_perp(&symbol, levels, EXCHANGE, true).await
        }
        JsonCommand::PerpBuy {
            symbol,
            amount,
            price,
            close,
        } => {
            commands::perp::buy(
                &symbol,
                &fmt_num(amount),
                &fmt_num(price),
                close,
                EXCHANGE,
                true,
            )
            .await
        }
        JsonCommand::PerpSell {
            symbol,
            amount,
            price,
            close,
        } => {
            commands::perp::sell(
                &symbol,
                &fmt_num(amount),
                &fmt_num(price),
                close,
                EXCHANGE,
                true,
            )
            .await
        }
        JsonCommand::PerpLeverage {
            symbol,
            leverage,
            cross,
        } => commands::perp::set_leverage(&symbol, leverage, cross, EXCHANGE, true).await,
        JsonCommand::PerpSetMode { mode } => commands::perp::set_mode(&mode, true).await,
        JsonCommand::OptionsBuy {
            symbol,
            option_type,
            strike,
            expiry,
            size,
        } => {
            commands::options::buy(
                &symbol,
                &option_type,
                &strike,
                &expiry,
                &size,
                EXCHANGE,
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
        } => {
            commands::options::sell(
                &symbol,
                &option_type,
                &strike,
                &expiry,
                &size,
                EXCHANGE,
                true,
            )
            .await
        }
        JsonCommand::Deposit {
            asset,
            amount,
            from,
            dry_run,
        } => {
            let amount = fmt_num(amount);
            commands::deposit::run(
                &asset,
                Some(amount.as_str()),
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
        JsonCommand::Transfer {
            asset,
            amount,
            from,
            to,
        } => commands::transfer::run(&asset, &fmt_num(amount), &from, &to, true).await,
        JsonCommand::BridgeStatus => commands::bridge_status::run(true).await,
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
            <Cli as clap::Parser>::parse_from(["hyperliquid", "--help"]);
            unreachable!()
        }
    };
    let json_output = false;

    let result = match command {
        Commands::Address => {
            let address = signing::get_wallet_address().context("No wallet configured")?;
            println!("{}", address);
            Ok(())
        }
        Commands::Quote { symbol } => commands::quote::run_perp(&symbol, json_output).await,
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
        Commands::Positions => commands::positions::run(EXCHANGE, json_output).await,
        Commands::Perp(cmd) => match cmd {
            PerpCmd::Quote { symbol } => commands::quote::run_perp(&symbol, json_output).await,
            PerpCmd::Orderbook { symbol, levels } => {
                commands::orderbook::run_perp(&symbol, levels, EXCHANGE, json_output).await
            }
            PerpCmd::Buy {
                symbol,
                amount,
                price,
                close,
            } => commands::perp::buy(&symbol, &amount, &price, close, EXCHANGE, json_output).await,
            PerpCmd::Sell {
                symbol,
                amount,
                price,
                close,
            } => commands::perp::sell(&symbol, &amount, &price, close, EXCHANGE, json_output).await,
            PerpCmd::Leverage {
                symbol,
                leverage,
                cross,
            } => {
                commands::perp::set_leverage(&symbol, leverage, cross, EXCHANGE, json_output).await
            }
            PerpCmd::SetMode { mode } => commands::perp::set_mode(&mode, json_output).await,
        },
        Commands::Options(cmd) => match cmd {
            OptionsCmd::Buy {
                symbol,
                option_type,
                strike,
                expiry,
                size,
            } => {
                commands::options::buy(
                    &symbol,
                    &option_type,
                    &strike,
                    &expiry,
                    &size,
                    EXCHANGE,
                    json_output,
                )
                .await
            }
            OptionsCmd::Sell {
                symbol,
                option_type,
                strike,
                expiry,
                size,
            } => {
                commands::options::sell(
                    &symbol,
                    &option_type,
                    &strike,
                    &expiry,
                    &size,
                    EXCHANGE,
                    json_output,
                )
                .await
            }
        },
        Commands::Deposit {
            asset,
            amount,
            from,
            dry_run,
        } => {
            commands::deposit::run(
                &asset,
                Some(amount.as_str()),
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
        Commands::Transfer {
            asset,
            amount,
            from,
            to,
        } => commands::transfer::run(&asset, &amount, &from, &to, json_output).await,
        Commands::BridgeStatus => commands::bridge_status::run(json_output).await,
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}
