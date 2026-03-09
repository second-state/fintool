use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::Deserialize;

use fintool_lib::{commands, format::fmt_num};

#[derive(Parser)]
#[command(
    name = "polymarket",
    about = "Polymarket trading CLI — prediction market trading on Polygon"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// JSON mode: pass a JSON command string for programmatic use (always outputs JSON).
    /// Example: polymarket --json '{"command":"buy","market":"will-btc-hit-100k","outcome":"yes","amount":10,"price":0.65}'
    #[arg(long)]
    json: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List/search prediction markets
    List {
        /// Search query
        #[arg(long)]
        query: Option<String>,
        /// Max results
        #[arg(long, default_value = "10")]
        limit: i32,
        /// Filter active only
        #[arg(long)]
        active: Option<bool>,
        /// Sort by: volume, liquidity
        #[arg(long)]
        sort: Option<String>,
        /// Minimum days from now before market closes (default: 3)
        #[arg(long, default_value = "3")]
        min_end_days: i64,
    },

    /// Get prediction market quote/details
    Quote {
        /// Market slug or ID
        market: String,
    },

    /// Buy shares in a prediction market outcome
    Buy {
        /// Market slug or condition ID
        market: String,
        /// Outcome: yes or no
        #[arg(long)]
        outcome: String,
        /// Amount in USDC
        #[arg(long)]
        amount: String,
        /// Max price (0.01-0.99)
        #[arg(long)]
        price: String,
    },

    /// Sell shares in a prediction market outcome
    Sell {
        /// Market slug or condition ID
        market: String,
        /// Outcome: yes or no
        #[arg(long)]
        outcome: String,
        /// Amount of shares to sell
        #[arg(long)]
        amount: String,
        /// Min price (0.01-0.99)
        #[arg(long)]
        price: String,
    },

    /// Show prediction market positions
    Positions,

    /// Deposit USDC to Polymarket
    Deposit {
        /// Amount of USDC to deposit
        #[arg(long)]
        amount: Option<String>,
        /// Source chain (e.g. base)
        #[arg(long)]
        from: Option<String>,
        /// Show quote only, don't execute
        #[arg(long)]
        dry_run: bool,
    },

    /// Withdraw USDC from Polymarket
    Withdraw {
        /// Amount of USDC to withdraw
        #[arg(long)]
        amount: String,
        /// Show quote only, don't execute
        #[arg(long)]
        dry_run: bool,
    },

    /// Show Polymarket USDC balance
    Balance,
}

// --- JSON mode ---

fn default_predict_limit() -> i32 {
    10
}

fn default_min_end_days() -> i64 {
    3
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum JsonCommand {
    List {
        query: Option<String>,
        #[serde(default = "default_predict_limit")]
        limit: i32,
        active: Option<bool>,
        sort: Option<String>,
        #[serde(default = "default_min_end_days")]
        min_end_days: i64,
    },
    Quote {
        market: String,
    },
    Buy {
        market: String,
        outcome: String,
        amount: f64,
        price: f64,
    },
    Sell {
        market: String,
        outcome: String,
        amount: f64,
        price: f64,
    },
    Positions,
    Deposit {
        amount: Option<f64>,
        from: Option<String>,
        #[serde(default)]
        dry_run: bool,
    },
    Withdraw {
        amount: f64,
        #[serde(default)]
        dry_run: bool,
    },
    Balance,
}

async fn run_json(json_str: &str) -> Result<()> {
    let cmd: JsonCommand = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON command: {}", e))?;

    match cmd {
        JsonCommand::List {
            query,
            limit,
            active,
            sort,
            min_end_days,
        } => {
            commands::predict::list(
                query.as_deref(),
                limit,
                active,
                sort.as_deref(),
                min_end_days,
                true,
            )
            .await
        }
        JsonCommand::Quote { market } => commands::predict::quote(&market, true).await,
        JsonCommand::Buy {
            market,
            outcome,
            amount,
            price,
        } => {
            commands::predict::buy(&market, &outcome, &fmt_num(amount), &fmt_num(price), true).await
        }
        JsonCommand::Sell {
            market,
            outcome,
            amount,
            price,
        } => {
            commands::predict::sell(&market, &outcome, &fmt_num(amount), &fmt_num(price), true)
                .await
        }
        JsonCommand::Positions => commands::predict::positions(true).await,
        JsonCommand::Deposit {
            amount,
            from,
            dry_run,
        } => {
            let amount = amount.map(fmt_num);
            commands::deposit::run(
                "USDC",
                amount.as_deref(),
                from.as_deref(),
                "polymarket",
                dry_run,
                true,
            )
            .await
        }
        JsonCommand::Withdraw { amount, dry_run } => {
            let amount = fmt_num(amount);
            commands::withdraw::run(&amount, "USDC", None, None, "polymarket", dry_run, true).await
        }
        JsonCommand::Balance => commands::balance::run("polymarket", true).await,
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
            <Cli as clap::Parser>::parse_from(["polymarket", "--help"]);
            unreachable!()
        }
    };
    let json_output = false;

    let result = match command {
        Commands::List {
            query,
            limit,
            active,
            sort,
            min_end_days,
        } => {
            commands::predict::list(
                query.as_deref(),
                limit,
                active,
                sort.as_deref(),
                min_end_days,
                json_output,
            )
            .await
        }
        Commands::Quote { market } => commands::predict::quote(&market, json_output).await,
        Commands::Buy {
            market,
            outcome,
            amount,
            price,
        } => commands::predict::buy(&market, &outcome, &amount, &price, json_output).await,
        Commands::Sell {
            market,
            outcome,
            amount,
            price,
        } => commands::predict::sell(&market, &outcome, &amount, &price, json_output).await,
        Commands::Positions => commands::predict::positions(json_output).await,
        Commands::Deposit {
            amount,
            from,
            dry_run,
        } => {
            commands::deposit::run(
                "USDC",
                amount.as_deref(),
                from.as_deref(),
                "polymarket",
                dry_run,
                json_output,
            )
            .await
        }
        Commands::Withdraw { amount, dry_run } => {
            commands::withdraw::run(
                &amount,
                "USDC",
                None,
                None,
                "polymarket",
                dry_run,
                json_output,
            )
            .await
        }
        Commands::Balance => commands::balance::run("polymarket", json_output).await,
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}
