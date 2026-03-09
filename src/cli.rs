use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fintool",
    about = "Financial trading CLI for crypto and stocks"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// JSON mode: pass a JSON command string for programmatic use (always outputs JSON).
    /// Example: fintool --json '{"command":"quote","symbol":"BTC"}'
    #[arg(long)]
    pub json: Option<String>,

    /// Exchange to use: hyperliquid, binance, or auto (default: auto)
    #[arg(long, global = true, default_value = "auto")]
    pub exchange: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize config file at ~/.fintool/config.toml
    Init,

    /// Print the configured wallet address
    Address,

    /// Get spot price quote (Hyperliquid spot → Yahoo Finance fallback)
    Quote { symbol: String },

    /// Get latest news for a symbol
    News { symbol: String },

    /// Spot limit orders (buy/sell)
    #[command(subcommand)]
    Order(OrderCmd),

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

    /// Perpetual futures trading
    #[command(subcommand)]
    Perp(PerpCmd),

    /// Options trading
    #[command(subcommand)]
    Options(OptionsCmd),

    /// Get stock reports (10-K annual, 10-Q quarterly) from SEC EDGAR
    #[command(subcommand)]
    Report(ReportCmd),

    /// Deposit to exchange: address for ETH/BTC/SOL, or bridge USDC
    Deposit {
        /// Asset: ETH, BTC, SOL, USDC, etc.
        asset: String,
        /// Amount (required for USDC bridge to Hyperliquid)
        #[arg(long)]
        amount: Option<String>,
        /// Source chain for USDC: ethereum or base
        #[arg(long)]
        from: Option<String>,
        /// Show quote only, don't execute transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Withdraw from exchange to external address
    Withdraw {
        /// Asset: ETH, BTC, SOL, USDC, etc.
        asset: String,
        /// Amount to withdraw (e.g. 10)
        #[arg(long)]
        amount: String,
        /// Destination: chain name (base, ethereum) or address (0x...)
        #[arg(long)]
        to: Option<String>,
        /// Network for Binance/Coinbase (e.g. ethereum, base, arbitrum, solana)
        #[arg(long)]
        network: Option<String>,
        /// Show quote only, don't execute
        #[arg(long)]
        dry_run: bool,
    },

    /// Transfer assets between perp, spot, and HIP-3 dex accounts on Hyperliquid
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

    /// Prediction market trading (Polymarket)
    #[command(subcommand)]
    Predict(PredictCmd),
}

#[derive(Subcommand)]
pub enum PredictCmd {
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
        /// Amount of the asset to buy (in symbol units, e.g. 1.0 HYPE)
        #[arg(long)]
        amount: String,
        /// Maximum price per unit (limit price)
        #[arg(long)]
        price: String,
    },
    /// Place a spot limit sell order (price is the minimum price you'll accept)
    Sell {
        symbol: String,
        /// Amount of the asset to sell (in symbol units)
        #[arg(long)]
        amount: String,
        /// Minimum price per unit (limit price)
        #[arg(long)]
        price: String,
    },
}

#[derive(Subcommand)]
pub enum PerpCmd {
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
