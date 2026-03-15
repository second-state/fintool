pub mod backtest;
pub mod binance;
pub mod bridge;
pub mod coinbase;
pub mod commands;
pub mod config;
pub mod format;
pub mod hip3;
pub mod okx;
pub mod polymarket;
pub mod signing;
pub mod unit;

/// Known chain names for withdraw --to detection
pub const KNOWN_CHAINS: &[&str] = &[
    "base",
    "ethereum",
    "eth",
    "mainnet",
    "arbitrum",
    "arb",
    "solana",
    "sol",
    "bitcoin",
    "btc",
    "bsc",
    "bnb",
    "polygon",
    "matic",
    "optimism",
    "op",
    "avalanche",
    "avax",
];

/// Resolve --to and --network for the withdraw command.
/// --to can be either a chain name or a destination address.
/// If --to is a recognized chain name and --network is not set, treat --to as the network.
pub fn resolve_withdraw_destination(
    to: Option<&str>,
    network: Option<&str>,
) -> (Option<String>, Option<String>) {
    match (to, network) {
        // Both specified: --to is address, --network is chain
        (Some(t), Some(n)) => (Some(t.to_string()), Some(n.to_string())),
        // Only --to: detect if it's a chain name or address
        (Some(t), None) => {
            if KNOWN_CHAINS.contains(&t.to_lowercase().as_str()) {
                (None, Some(t.to_string()))
            } else {
                (Some(t.to_string()), None)
            }
        }
        // Only --network
        (None, Some(n)) => (None, Some(n.to_string())),
        // Neither
        (None, None) => (None, None),
    }
}
