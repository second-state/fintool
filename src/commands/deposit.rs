//! `fintool deposit <amount> <asset>` — generate a Unit deposit address and show instructions
//! For ETH/BTC/SOL: uses Unit bridge (native chain → Hyperliquid)
//! For USDC: uses Hyperliquid's Arbitrum bridge (HL SDK)

use anyhow::{bail, Result};
use colored::Colorize;
use serde_json::json;

use crate::config;
use crate::unit;

pub async fn run(amount: &str, asset: &str, json_out: bool) -> Result<()> {
    let asset_lower = asset.to_lowercase();

    if asset_lower == "usdc" {
        return deposit_usdc(amount, json_out).await;
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

    // Generate deposit address: native chain → hyperliquid
    let resp =
        unit::generate_address(chain, "hyperliquid", &asset_lower, &cfg.address, cfg.testnet)
            .await?;

    // Estimate fees
    let fees = unit::estimate_fees(cfg.testnet).await.ok();

    if json_out {
        let mut out = json!({
            "action": "deposit",
            "asset": asset.to_uppercase(),
            "amount": amount,
            "source_chain": chain,
            "destination": "hyperliquid",
            "hl_address": cfg.address,
            "deposit_address": resp.address,
            "minimum": min,
            "instructions": format!(
                "Send {} {} on {} to {}",
                amount,
                asset.to_uppercase(),
                chain,
                resp.address
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
            "  {} {} {} → Hyperliquid",
            "Deposit".green().bold(),
            amount,
            asset.to_uppercase().cyan()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!(
            "  {} {}",
            "Source chain:".dimmed(),
            chain.yellow()
        );
        println!(
            "  {} {}",
            "HL address:  ".dimmed(),
            cfg.address.cyan()
        );
        println!(
            "  {} {}",
            "Minimum:     ".dimmed(),
            min
        );
        println!();
        println!(
            "  {} Send {} {} on {} to:",
            "→".green().bold(),
            amount,
            asset.to_uppercase(),
            chain
        );
        println!();
        println!("    {}", resp.address.green().bold());
        println!();
        if let Some(ref f) = fees {
            let key_deposit = format!("{}-depositEta", chain);
            let key_fee = format!("{}-depositFee", chain);
            if let Some(eta) = f.get(chain).and_then(|c| c.get(&key_deposit)) {
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
            "  {} This address is permanent for your HL wallet.",
            "ℹ".blue()
        );
        println!(
            "  {} Track status: fintool bridge-status",
            "ℹ".blue()
        );
        println!();
    }

    Ok(())
}

async fn deposit_usdc(amount: &str, json_out: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;

    if json_out {
        let out = json!({
            "action": "deposit",
            "asset": "USDC",
            "amount": amount,
            "source_chain": "arbitrum",
            "destination": "hyperliquid",
            "hl_address": cfg.address,
            "instructions": format!(
                "Send {} USDC on Arbitrum to the Hyperliquid bridge contract. \
                 Use the Hyperliquid web UI at https://app.hyperliquid.xyz or \
                 the HL SDK's deposit method.",
            amount),
            "note": "USDC deposits use Hyperliquid's native Arbitrum bridge, not Unit.",
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} {} → Hyperliquid",
            "Deposit".green().bold(),
            amount,
            "USDC".cyan()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!(
            "  {} Arbitrum (native HL bridge, not Unit)",
            "Source chain:".dimmed()
        );
        println!(
            "  {} {}",
            "HL address:  ".dimmed(),
            cfg.address.cyan()
        );
        println!();
        println!(
            "  {} USDC deposits go through Hyperliquid's native Arbitrum bridge.",
            "ℹ".blue()
        );
        println!(
            "  {} Use https://app.hyperliquid.xyz → Deposit",
            "→".green().bold()
        );
        println!(
            "  {} Or send USDC on Arbitrum to the HL bridge contract.",
            "→".green().bold()
        );
        println!();
    }

    Ok(())
}
