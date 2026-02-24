//! `fintool deposit <asset>` — deposit to Hyperliquid
//! - ETH/BTC/SOL: generates a permanent Unit deposit address (no amount needed)
//! - USDC: bridges USDC from Ethereum/Base → Arbitrum → HL (amount + --from required)

use anyhow::{bail, Result};
use colored::Colorize;
use serde_json::json;

use crate::bridge::{self, SourceChain};
use crate::config;
use crate::unit;

pub async fn run(
    asset: &str,
    amount: Option<&str>,
    from: Option<&str>,
    json_out: bool,
) -> Result<()> {
    let asset_lower = asset.to_lowercase();

    if asset_lower == "usdc" {
        return deposit_usdc(amount, from, json_out).await;
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
            "source_chain": chain,
            "destination": "hyperliquid",
            "hl_address": cfg.address,
            "deposit_address": resp.address,
            "minimum": min,
            "instructions": format!(
                "Send {} on {} to {} (any amount above minimum, reusable address)",
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
            "  {} {} → Hyperliquid",
            "Deposit".green().bold(),
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
            "  {} Send {} on {} to:",
            "→".green().bold(),
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
            "  {} This address is permanent — send any amount, any time.",
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

async fn deposit_usdc(
    amount: Option<&str>,
    from: Option<&str>,
    json_out: bool,
) -> Result<()> {
    let cfg = config::load_hl_config()?;

    let amount = match amount {
        Some(a) => a,
        None => bail!(
            "USDC deposits require an amount.\n\
             Usage: fintool deposit USDC --amount 100 --from ethereum\n\
             \n\
             Supported source chains: ethereum, base"
        ),
    };

    let from = match from {
        Some(f) => f,
        None => bail!(
            "USDC deposits require a source chain.\n\
             Usage: fintool deposit USDC --amount 100 --from ethereum\n\
             \n\
             Supported source chains: ethereum, base"
        ),
    };

    let source: SourceChain = from.parse()?;

    // Step 1: Get Across bridge quote
    eprintln!("Fetching bridge quote from Across...");
    let quote = bridge::get_across_quote(source, amount, &cfg.address).await?;

    let output_amount = quote
        .expected_output_amount
        .as_deref()
        .unwrap_or(&quote.input_amount);
    let fill_time = quote.expected_fill_time.unwrap_or(0);
    let needs_approval = quote.approval_txns.is_some()
        && !quote.approval_txns.as_ref().unwrap().is_empty();

    if json_out {
        let mut out = json!({
            "action": "deposit_usdc",
            "source_chain": source.name(),
            "destination": "hyperliquid",
            "amount_in": bridge::format_usdc(&quote.input_amount),
            "amount_out_arbitrum": bridge::format_usdc(output_amount),
            "expected_fill_time_seconds": fill_time,
            "needs_approval": needs_approval,
            "hl_address": cfg.address,
            "steps": [],
        });

        let steps = out["steps"].as_array_mut().unwrap();

        if needs_approval {
            for (i, tx) in quote.approval_txns.as_ref().unwrap().iter().enumerate() {
                steps.push(json!({
                    "step": format!("approve_{}", i + 1),
                    "chain": source.name(),
                    "chain_id": source.chain_id(),
                    "to": tx.to,
                    "data": tx.data,
                    "description": "ERC-20 approval for Across SpokePool",
                }));
            }
        }

        steps.push(json!({
            "step": "bridge",
            "chain": source.name(),
            "chain_id": source.chain_id(),
            "to": quote.swap_tx.to,
            "data": quote.swap_tx.data,
            "value": quote.swap_tx.value,
            "description": format!(
                "Bridge {} from {} to Arbitrum via Across (~{}s)",
                bridge::format_usdc(&quote.input_amount),
                source.name(),
                fill_time
            ),
        }));

        steps.push(json!({
            "step": "hl_deposit",
            "chain": "arbitrum",
            "chain_id": bridge::ARBITRUM_CHAIN_ID,
            "to": bridge::HL_BRIDGE2_MAINNET,
            "description": format!(
                "Send {} to HL Bridge2 on Arbitrum (auto-credited to {})",
                bridge::format_usdc(output_amount),
                cfg.address
            ),
        }));

        if let Some(ref fees) = quote.fees {
            out["fees"] = fees.clone();
        }

        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", "━".repeat(55).dimmed());
        println!(
            "  {} {} USDC  {} → Hyperliquid",
            "Deposit".green().bold(),
            amount,
            source.name().yellow()
        );
        println!("{}", "━".repeat(55).dimmed());
        println!();
        println!(
            "  {} {} → Arbitrum → Hyperliquid",
            "Route:      ".dimmed(),
            source.name()
        );
        println!(
            "  {} {}",
            "Bridge:     ".dimmed(),
            "Across Protocol"
        );
        println!(
            "  {} {}",
            "Amount in:  ".dimmed(),
            bridge::format_usdc(&quote.input_amount).cyan()
        );
        println!(
            "  {} {}",
            "Amount out: ".dimmed(),
            bridge::format_usdc(output_amount).green()
        );
        println!(
            "  {} ~{}s",
            "Fill time:  ".dimmed(),
            fill_time
        );
        println!(
            "  {} {}",
            "HL address: ".dimmed(),
            cfg.address.cyan()
        );

        println!();
        println!("  {}", "Transaction Steps:".bold());

        let mut step = 1;
        if needs_approval {
            for tx in quote.approval_txns.as_ref().unwrap() {
                println!(
                    "  {}. {} USDC approval on {} → {}",
                    step,
                    "Approve".yellow(),
                    source.name(),
                    &tx.to[..10]
                );
                step += 1;
            }
        }

        println!(
            "  {}. {} {} via Across SpokePool on {}",
            step,
            "Bridge".green(),
            bridge::format_usdc(&quote.input_amount),
            source.name()
        );
        step += 1;

        println!(
            "  {}. {} {} to HL Bridge2 on Arbitrum",
            step,
            "Deposit".green(),
            bridge::format_usdc(output_amount)
        );

        println!();
        println!(
            "  {} All transactions use your configured private key.",
            "ℹ".blue()
        );
        println!(
            "  {} RPC: {} (source), {} (Arbitrum)",
            "ℹ".blue(),
            source.rpc_url(),
            bridge::RPC_ARBITRUM
        );
        println!();
        println!(
            "  {} To execute, run: fintool deposit USDC --amount {} --from {} --execute",
            "→".green().bold(),
            amount,
            source.name()
        );
        println!();
    }

    Ok(())
}
