mod binance;
mod bridge;
mod cli;
mod coinbase;
mod commands;
mod config;
mod format;
mod hip3;
mod json_dispatch;
mod signing;
mod unit;

use anyhow::{Context, Result};
use cli::{Cli, Commands, OptionsCmd, OrderCmd, PerpCmd, ReportCmd};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = <Cli as clap::Parser>::parse();

    // JSON mode: parse JSON string, dispatch with json_output=true, output errors as JSON
    if let Some(ref json_str) = cli.json {
        let result = json_dispatch::run(json_str).await;
        if let Err(e) = result {
            let err_json = serde_json::json!({"error": format!("{:#}", e)});
            println!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            std::process::exit(1);
        }
        return Ok(());
    }

    // Normal CLI mode: human-readable output
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // No subcommand and no --json: print help and exit
            <Cli as clap::Parser>::parse_from(["fintool", "--help"]);
            unreachable!()
        }
    };
    let json_output = false;

    let result = match command {
        Commands::Init => match config::init_config() {
            Ok((path, created)) => {
                if created {
                    println!("Config file created at: {}", path.display());
                    println!("Edit it to add your wallet and API keys.");
                } else {
                    println!("Config file already exists at: {}", path.display());
                    println!("Not overwriting. Edit it directly to make changes.");
                }
                Ok(())
            }
            Err(e) => Err(e),
        },
        Commands::Address => {
            let address = signing::get_wallet_address().context("No wallet configured")?;
            println!("{}", address);
            Ok(())
        }
        Commands::Quote { symbol } => commands::quote::run_spot(&symbol, json_output).await,
        Commands::News { symbol } => commands::news::run(&symbol, json_output).await,
        Commands::Order(cmd) => match cmd {
            OrderCmd::Buy {
                symbol,
                amount,
                price,
            } => {
                commands::order::buy(&symbol, &amount, &price, &cli.exchange, json_output).await
            }
            OrderCmd::Sell {
                symbol,
                amount,
                price,
            } => {
                commands::order::sell(&symbol, &amount, &price, &cli.exchange, json_output).await
            }
        },
        Commands::Orders { symbol } => {
            commands::orders::run(symbol.as_deref(), &cli.exchange, json_output).await
        }
        Commands::Cancel { order_id } => {
            commands::cancel::run(&order_id, &cli.exchange, json_output).await
        }
        Commands::Balance => commands::balance::run(&cli.exchange, json_output).await,
        Commands::Positions => commands::positions::run(&cli.exchange, json_output).await,
        Commands::Perp(cmd) => match cmd {
            PerpCmd::Quote { symbol } => commands::quote::run_perp(&symbol, json_output).await,
            PerpCmd::Buy {
                symbol,
                amount,
                price,
                close,
            } => {
                commands::perp::buy(&symbol, &amount, &price, close, &cli.exchange, json_output)
                    .await
            }
            PerpCmd::Sell {
                symbol,
                amount,
                price,
                close,
            } => {
                commands::perp::sell(&symbol, &amount, &price, close, &cli.exchange, json_output)
                    .await
            }
            PerpCmd::Leverage {
                symbol,
                leverage,
                cross,
            } => {
                commands::perp::set_leverage(&symbol, leverage, cross, &cli.exchange, json_output)
                    .await
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
                    &cli.exchange,
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
                    &cli.exchange,
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
                amount.as_deref(),
                from.as_deref(),
                &cli.exchange,
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
                &cli.exchange,
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
        } => {
            if cli.exchange != "auto" && cli.exchange != "hyperliquid" {
                anyhow::bail!(
                    "Transfer between perp and spot is only supported on Hyperliquid. Got --exchange {}",
                    cli.exchange
                );
            }
            commands::transfer::run(&asset, &amount, &from, &to, json_output).await
        }
        Commands::BridgeStatus => commands::bridge_status::run(json_output).await,
        Commands::Report(cmd) => match cmd {
            ReportCmd::Annual { symbol, output } => {
                commands::report::annual(&symbol, output.as_deref(), json_output).await
            }
            ReportCmd::Quarterly { symbol, output } => {
                commands::report::quarterly(&symbol, output.as_deref(), json_output).await
            }
            ReportCmd::List { symbol, limit } => {
                commands::report::list(&symbol, limit, json_output).await
            }
            ReportCmd::Get {
                symbol,
                accession,
                output,
            } => {
                commands::report::get(&symbol, &accession, output.as_deref(), json_output).await
            }
        },
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}

use colored::Colorize;

/// Known chain names for withdraw --to detection
const KNOWN_CHAINS: &[&str] = &[
    "base", "ethereum", "eth", "mainnet", "arbitrum", "arb",
    "solana", "sol", "bitcoin", "btc", "bsc", "bnb",
    "polygon", "matic", "optimism", "op", "avalanche", "avax",
];

/// Resolve --to and --network for the withdraw command.
/// --to can be either a chain name or a destination address.
/// If --to is a recognized chain name and --network is not set, treat --to as the network.
pub(crate) fn resolve_withdraw_destination(
    to: Option<&str>,
    network: Option<&str>,
) -> (Option<String>, Option<String>) {
    match (to, network) {
        // Both specified: --to is address, --network is chain
        (Some(t), Some(n)) => (Some(t.to_string()), Some(n.to_string())),
        // Only --to: detect if it's a chain name or address
        (Some(t), None) => {
            if KNOWN_CHAINS.contains(&t.to_lowercase().as_str()) {
                // --to is a chain name → use as network, no explicit address
                (None, Some(t.to_string()))
            } else {
                // --to is an address
                (Some(t.to_string()), None)
            }
        }
        // Only --network
        (None, Some(n)) => (None, Some(n.to_string())),
        // Neither
        (None, None) => (None, None),
    }
}
