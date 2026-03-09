use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::Deserialize;

use fintool_lib::{commands, config};

#[derive(Parser)]
#[command(
    name = "fintool",
    about = "Market intelligence CLI — prices, news, and SEC filings"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// JSON mode: pass a JSON command string for programmatic use (always outputs JSON).
    /// Example: fintool --json '{"command":"quote","symbol":"BTC"}'
    #[arg(long)]
    json: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize config file at ~/.fintool/config.toml
    Init,

    /// Get spot price quote (multi-source: Hyperliquid, Yahoo Finance, CoinGecko)
    Quote { symbol: String },

    /// Get latest news for a symbol
    News { symbol: String },

    /// Get stock reports (10-K annual, 10-Q quarterly) from SEC EDGAR
    #[command(subcommand)]
    Report(ReportCmd),
}

#[derive(Subcommand)]
enum ReportCmd {
    /// Get the latest annual report (10-K)
    Annual {
        symbol: String,
        /// Save report to file
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Get the latest quarterly report (10-Q)
    Quarterly {
        symbol: String,
        /// Save report to file
        #[arg(long, short)]
        output: Option<String>,
    },
    /// List recent filings
    List {
        symbol: String,
        /// Number of filings to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Get a specific filing by accession number
    Get {
        symbol: String,
        /// Accession number (e.g. 0001628280-26-003952)
        accession: String,
        /// Save report to file
        #[arg(long, short)]
        output: Option<String>,
    },
}

// --- JSON mode ---

fn default_limit() -> usize {
    10
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum JsonCommand {
    Init,
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
}

async fn run_json(json_str: &str) -> Result<()> {
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
        JsonCommand::Quote { symbol } => commands::quote::run_spot(&symbol, true).await,
        JsonCommand::News { symbol } => commands::news::run(&symbol, true).await,
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
        Commands::Quote { symbol } => commands::quote::run_spot(&symbol, json_output).await,
        Commands::News { symbol } => commands::news::run(&symbol, json_output).await,
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
            } => commands::report::get(&symbol, &accession, output.as_deref(), json_output).await,
        },
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", "Error".red(), e);
        std::process::exit(1);
    }
    Ok(())
}
