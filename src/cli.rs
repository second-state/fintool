use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fintool", about = "Financial trading CLI for crypto and stocks")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Human-friendly colored output (default is JSON)
    #[arg(long, global = true)]
    pub human: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize config file at ~/.fintool/config.toml
    Init,

    /// Get spot price quote (Hyperliquid spot → Yahoo Finance fallback)
    Quote {
        symbol: String,
    },

    /// Get latest news for a symbol
    News { symbol: String },

    /// Spot limit orders (buy/sell)
    #[command(subcommand)]
    Order(OrderCmd),

    /// List open orders (spot and perp)
    Orders { symbol: Option<String> },

    /// Cancel an order
    Cancel { order_id: String },

    /// Show account balances
    Balance,

    /// Show open positions
    Positions,

    /// Perpetual futures trading
    #[command(subcommand)]
    Perp(PerpCmd),

    /// Options trading
    #[command(subcommand)]
    Options(OptionsCmd),

    /// Prediction market trading
    #[command(subcommand)]
    Predict(PredictCmd),

    /// Get stock reports (10-K annual, 10-Q quarterly) from SEC EDGAR
    #[command(subcommand)]
    Report(ReportCmd),
}

#[derive(Subcommand)]
pub enum ReportCmd {
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

#[derive(Subcommand)]
pub enum OrderCmd {
    /// Place a spot limit buy order (price is the maximum price you'll pay)
    Buy {
        symbol: String,
        /// Amount in USDC to spend
        amount_usdc: String,
        /// Maximum price per unit (limit price)
        max_price: String,
    },
    /// Place a spot limit sell order (price is the minimum price you'll accept)
    Sell {
        symbol: String,
        /// Amount of the asset to sell
        amount: String,
        /// Minimum price per unit (limit price)
        min_price: String,
    },
}

#[derive(Subcommand)]
pub enum PerpCmd {
    /// Get perpetual futures price quote
    Quote { symbol: String },
    /// Place a perp limit buy (long) order
    Buy {
        symbol: String,
        /// Amount in USDC
        amount_usdc: String,
        /// Limit price
        price: String,
    },
    /// Place a perp limit sell (short) order
    Sell {
        symbol: String,
        /// Size in asset units
        amount: String,
        /// Limit price
        price: String,
    },
}

#[derive(Subcommand)]
pub enum OptionsCmd {
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

#[derive(Subcommand)]
pub enum PredictCmd {
    /// List trending/popular prediction markets
    List {
        /// Filter by platform: polymarket, kalshi, or all (default)
        #[arg(long, default_value = "all")]
        platform: String,
        /// Max results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Search prediction markets by keyword
    Search {
        query: String,
        #[arg(long, default_value = "all")]
        platform: String,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Get price/probability quote for a specific market
    Quote {
        /// Market ID or ticker (e.g. polymarket:slug or kalshi:TICKER)
        market: String,
    },
    /// Buy a prediction contract
    Buy {
        /// Market ID (polymarket:slug or kalshi:TICKER)
        market: String,
        /// Side: yes or no
        side: String,
        /// Amount in USDC (Polymarket) or USD (Kalshi)
        amount: String,
        /// Max price in cents (1-99)
        #[arg(long)]
        max_price: Option<String>,
    },
    /// Sell a prediction contract
    Sell {
        /// Market ID (polymarket:slug or kalshi:TICKER)
        market: String,
        /// Side: yes or no
        side: String,
        /// Number of contracts to sell
        amount: String,
        /// Min price in cents (1-99)
        #[arg(long)]
        min_price: Option<String>,
    },
}
