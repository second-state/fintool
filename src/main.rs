mod binance;
mod bridge;
mod cli;
mod coinbase;
mod hip3;
mod commands;
mod config;
mod format;
mod polymarket;
mod signing;
mod unit;

use anyhow::Result;
use cli::{Cli, Commands, OptionsCmd, OrderCmd, PerpCmd, PredictCmd, ReportCmd};

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
            } => commands::perp::sell(&symbol, &amount, &price, &cli.exchange, json).await,
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
        Commands::Predict(cmd) => match cmd {
            PredictCmd::List { platform, limit } => {
                commands::predict::list(&platform, limit, json).await
            }
            PredictCmd::Search {
                query,
                platform,
                limit,
            } => commands::predict::search(&query, &platform, limit, json).await,
            PredictCmd::Quote { market } => commands::predict::quote(&market, json).await,
            PredictCmd::Buy {
                market,
                side,
                amount,
                max_price,
            } => commands::predict::buy(&market, &side, &amount, max_price.as_deref(), json).await,
            PredictCmd::Sell {
                market,
                side,
                amount,
                min_price,
            } => commands::predict::sell(&market, &side, &amount, min_price.as_deref(), json).await,
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
