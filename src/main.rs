mod binance;
mod bridge;
mod cli;
mod coinbase;
mod commands;
mod config;
mod format;
mod hip3;
mod signing;
mod unit;

use anyhow::{Context, Result};
use cli::{Cli, Commands, OptionsCmd, OrderCmd, PerpCmd, ReportCmd};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = <Cli as clap::Parser>::parse();
    let json = !cli.human;

    let result = match cli.command {
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
            if json {
                println!("{}", serde_json::json!({"address": address}));
            } else {
                println!("{}", address);
            }
            Ok(())
        }
        Commands::Quote { symbol } => commands::quote::run_spot(&symbol, json).await,
        Commands::News { symbol } => commands::news::run(&symbol, json).await,
        Commands::Order(cmd) => match cmd {
            OrderCmd::Buy {
                symbol,
                amount_usdc,
                max_price,
            } => commands::order::buy(&symbol, &amount_usdc, &max_price, &cli.exchange, json).await,
            OrderCmd::Sell {
                symbol,
                amount,
                min_price,
            } => commands::order::sell(&symbol, &amount, &min_price, &cli.exchange, json).await,
        },
        Commands::Orders { symbol } => {
            commands::orders::run(symbol.as_deref(), &cli.exchange, json).await
        }
        Commands::Cancel { order_id } => {
            commands::cancel::run(&order_id, &cli.exchange, json).await
        }
        Commands::Balance => commands::balance::run(&cli.exchange, json).await,
        Commands::Positions => commands::positions::run(&cli.exchange, json).await,
        Commands::Perp(cmd) => match cmd {
            PerpCmd::Quote { symbol } => commands::quote::run_perp(&symbol, json).await,
            PerpCmd::Buy {
                symbol,
                amount_usdc,
                price,
            } => commands::perp::buy(&symbol, &amount_usdc, &price, &cli.exchange, json).await,
            PerpCmd::Sell {
                symbol,
                amount,
                price,
                close,
            } => commands::perp::sell(&symbol, &amount, &price, close, &cli.exchange, json).await,
            PerpCmd::Leverage {
                symbol,
                leverage,
                cross,
            } => commands::perp::set_leverage(&symbol, leverage, cross, &cli.exchange, json).await,
            PerpCmd::SetMode { mode } => commands::perp::set_mode(&mode, json).await,
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
                    json,
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
                    json,
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
                json,
            )
            .await
        }
        Commands::Withdraw {
            amount,
            asset,
            to,
            network,
            dry_run,
        } => {
            commands::withdraw::run(
                &amount,
                &asset,
                to.as_deref(),
                network.as_deref(),
                &cli.exchange,
                dry_run,
                json,
            )
            .await
        }
        Commands::Transfer { amount, direction, dex } => {
            if cli.exchange != "auto" && cli.exchange != "hyperliquid" {
                anyhow::bail!(
                    "Transfer between perp and spot is only supported on Hyperliquid. Got --exchange {}",
                    cli.exchange
                );
            }
            config::load_hl_config().context(
                "Hyperliquid wallet not configured. Transfer requires Hyperliquid."
            )?;
            let amount_f: f64 = amount.parse().map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount))?;

            match direction.as_str() {
                "to-perp" | "to-spot" => {
                    let to_perp = direction == "to-perp";
                    let dir_label = if to_perp { "spot → perp" } else { "perp → spot" };
                    signing::class_transfer(amount_f, to_perp).await?;
                    if json {
                        println!("{}", serde_json::json!({
                            "action": "transfer",
                            "amount": amount,
                            "direction": direction,
                            "status": "ok",
                        }));
                    } else {
                        println!("  Transferred ${} USDC ({})", amount, dir_label);
                    }
                }
                "to-dex" | "from-dex" => {
                    let dex_name = dex.as_deref().ok_or_else(||
                        anyhow::anyhow!("--dex is required for {} (e.g. --dex cash)", direction)
                    )?;
                    let (collateral_token, token_name) = signing::get_dex_collateral_token(dex_name).await?;
                    let (source, dest, dir_label) = if direction == "to-dex" {
                        ("spot", dex_name, format!("spot → {} dex", dex_name))
                    } else {
                        (dex_name, "spot", format!("{} dex → spot", dex_name))
                    };
                    signing::send_asset(amount_f, source, dest, &collateral_token).await?;
                    if json {
                        println!("{}", serde_json::json!({
                            "action": "transfer",
                            "amount": amount,
                            "direction": direction,
                            "dex": dex_name,
                            "token": token_name,
                            "status": "ok",
                        }));
                    } else {
                        println!("  Transferred ${} {} ({})", amount, token_name, dir_label);
                    }
                }
                _ => anyhow::bail!(
                    "Invalid direction: {}. Use 'to-spot', 'to-perp', 'to-dex', or 'from-dex'",
                    direction
                ),
            }
            Ok(())
        }
        Commands::BridgeStatus => commands::bridge_status::run(json).await,
        Commands::Report(cmd) => match cmd {
            ReportCmd::Annual { symbol, output } => {
                commands::report::annual(&symbol, output.as_deref(), json).await
            }
            ReportCmd::Quarterly { symbol, output } => {
                commands::report::quarterly(&symbol, output.as_deref(), json).await
            }
            ReportCmd::List { symbol, limit } => commands::report::list(&symbol, limit, json).await,
            ReportCmd::Get {
                symbol,
                accession,
                output,
            } => commands::report::get(&symbol, &accession, output.as_deref(), json).await,
        },
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}

use colored::Colorize;
