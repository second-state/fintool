//! `fintool withdraw <amount> <asset>` — generate a Unit withdrawal address and show instructions
//! For ETH/BTC/SOL: uses Unit bridge (Hyperliquid → native chain)
//! For USDC: uses Hyperliquid's native withdrawal (HL → Arbitrum)

use anyhow::{bail, Result};
use colored::Colorize;
use serde_json::json;

use crate::config;
use crate::unit;

pub async fn run(amount: &str, asset: &str, destination: Option<&str>, json_out: bool) -> Result<()> {
    let asset_lower = asset.to_lowercase();

    if asset_lower == "usdc" {
        return withdraw_usdc(amount, json_out).await;
    }

    if !unit::is_supported(&asset_lower) {
        bail!(
            "Unsupported asset '{}'. Supported: ETH, BTC, SOL, USDC",
            asset
        );
    }

    let cfg = config::load_hl_config()?;
    let chain = unit::native_chain(&asset_lower).unwrap();
    let min = unit::minimum_amount(&asset_lower).unwrap_or("unknown");

    // For withdrawals, the destination address on the native chain
    // defaults to the same address derived from the private key (for ETH/SOL)
    // For BTC, user must provide a destination address
    let dst_addr = match destination {
        Some(addr) => addr.to_string(),
        None => {
            if asset_lower == "btc" {
                bail!(
                    "BTC withdrawals require a destination address.\n\
                     Usage: fintool withdraw <amount> BTC --to <bitcoin_address>"
                );
            }
            // For ETH/SOL, default to the same address (works for ETH, may differ for SOL)
            cfg.address.clone()
        }
    };

    // Generate withdrawal address: hyperliquid → native chain
    let resp =
        unit::generate_address("hyperliquid", chain, &asset_lower, &dst_addr, cfg.testnet)
            .await?;

    // Estimate fees
    let fees = unit::estimate_fees(cfg.testnet).await.ok();

    if json_out {
        let mut out = json!({
            "action": "withdraw",
            "asset": asset.to_uppercase(),
            "amount": amount,
            "source": "hyperliquid",
            "destination_chain": chain,
            "destination_address": dst_addr,
            "unit_withdraw_address": resp.address,
            "minimum": min,
            "instructions": format!(
                "Send {} u{} on Hyperliquid to {} (Unit withdrawal address). \
                 Funds will arrive at {} on {}.",
                amount,
                asset.to_uppercase(),
                resp.address,
                dst_addr,
                chain,
            ),
        });
        if let Some(ref f) = fees {
            if let Some(chain_fees) = f.get(chain) {
                out["estimated_fees"] = chain_fees.clone();
            }
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} u{} → {}",
            "Withdraw".red().bold(),
            amount,
            asset.to_uppercase().cyan(),
            chain.yellow()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!(
            "  {} {}",
            "Dest chain:  ".dimmed(),
            chain.yellow()
        );
        println!(
            "  {} {}",
            "Dest address:".dimmed(),
            dst_addr.cyan()
        );
        println!(
            "  {} {}",
            "Minimum:     ".dimmed(),
            min
        );
        println!();
        println!(
            "  {} Transfer {} u{} on Hyperliquid to:",
            "→".red().bold(),
            amount,
            asset.to_uppercase(),
        );
        println!();
        println!("    {}", resp.address.red().bold());
        println!();
        println!(
            "  {} This is a Unit withdrawal address on Hyperliquid.",
            "ℹ".blue()
        );
        println!(
            "  {} Use Hyperliquid L1 spot transfer to send u{} to it.",
            "ℹ".blue(),
            asset.to_uppercase()
        );
        if let Some(ref f) = fees {
            let key_eta = format!("{}-withdrawalEta", chain);
            let key_fee = format!("{}-withdrawalFee", chain);
            if let Some(eta) = f.get(chain).and_then(|c| c.get(&key_eta)) {
                println!(
                    "  {} ~{}",
                    "Est. time:   ".dimmed(),
                    eta.as_str().unwrap_or("unknown")
                );
            }
            if let Some(fee) = f.get(chain).and_then(|c| c.get(&key_fee)) {
                let fee_str = unit::format_amount(
                    &fee.as_f64().unwrap_or(0.0).to_string(),
                    &asset_lower,
                );
                println!(
                    "  {} {}",
                    "Est. fee:    ".dimmed(),
                    fee_str
                );
            }
        }
        println!();
        println!(
            "  {} Track status: fintool bridge-status",
            "ℹ".blue()
        );
        println!();
    }

    Ok(())
}

async fn withdraw_usdc(amount: &str, json_out: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;

    if json_out {
        let out = json!({
            "action": "withdraw",
            "asset": "USDC",
            "amount": amount,
            "source": "hyperliquid",
            "destination_chain": "arbitrum",
            "destination_address": cfg.address,
            "instructions": format!(
                "Use Hyperliquid's native withdrawal to send {} USDC to {} on Arbitrum. \
                 Use the web UI at https://app.hyperliquid.xyz or the HL SDK.",
                amount, cfg.address
            ),
            "note": "USDC withdrawals use Hyperliquid's native Arbitrum bridge, not Unit.",
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} {} → Arbitrum",
            "Withdraw".red().bold(),
            amount,
            "USDC".cyan()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!(
            "  {} {}",
            "Dest address:".dimmed(),
            cfg.address.cyan()
        );
        println!();
        println!(
            "  {} USDC withdrawals use Hyperliquid's native Arbitrum bridge.",
            "ℹ".blue()
        );
        println!(
            "  {} Use https://app.hyperliquid.xyz → Withdraw",
            "→".red().bold()
        );
        println!();
    }

    Ok(())
}
