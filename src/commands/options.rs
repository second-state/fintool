use anyhow::Result;
use colored::Colorize;

pub async fn buy(
    symbol: &str,
    option_type: &str,
    strike: &str,
    expiry: &str,
    size: &str,
    json_output: bool,
) -> Result<()> {
    if json_output {
        println!(
            "{}",
            serde_json::json!({
                "status": "not_implemented",
                "note": "Native options support coming with Hyperliquid HIP-4",
                "params": { "symbol": symbol, "type": option_type, "strike": strike, "expiry": expiry, "size": size }
            })
        );
    } else {
        println!();
        println!("  📋 Options Buy (Stub)");
        println!("  Symbol: {}", symbol.cyan());
        println!("  Type:   {}", option_type);
        println!("  Strike: ${}", strike);
        println!("  Expiry: {}", expiry);
        println!("  Size:   {}", size);
        println!();
        println!(
            "  {} Native options support coming with Hyperliquid HIP-4.",
            "ℹ️".blue()
        );
        println!("  Currently, options-like exposure can be achieved via perps with stop-losses.");
        println!();
    }
    Ok(())
}

pub async fn sell(
    symbol: &str,
    option_type: &str,
    strike: &str,
    expiry: &str,
    size: &str,
    json_output: bool,
) -> Result<()> {
    if json_output {
        println!(
            "{}",
            serde_json::json!({
                "status": "not_implemented",
                "note": "Native options support coming with Hyperliquid HIP-4",
                "params": { "symbol": symbol, "type": option_type, "strike": strike, "expiry": expiry, "size": size }
            })
        );
    } else {
        println!();
        println!("  📋 Options Sell (Stub)");
        println!("  Symbol: {}", symbol.cyan());
        println!("  Type:   {}", option_type);
        println!("  Strike: ${}", strike);
        println!("  Expiry: {}", expiry);
        println!("  Size:   {}", size);
        println!();
        println!(
            "  {} Native options support coming with Hyperliquid HIP-4.",
            "ℹ️".blue()
        );
        println!();
    }
    Ok(())
}
