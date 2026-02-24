//! `fintool bridge-status` — show all Unit bridge operations for the configured wallet

use anyhow::Result;
use colored::Colorize;
use serde_json::json;

use crate::config;
use crate::unit;

pub async fn run(json_out: bool) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let ops = unit::get_operations(&cfg.address, cfg.testnet).await?;

    if json_out {
        let out = json!({
            "address": cfg.address,
            "addresses": ops.addresses,
            "operations": ops.operations.iter().map(|op| json!({
                "id": op.operation_id,
                "created_at": op.op_created_at,
                "asset": op.asset,
                "source_chain": op.source_chain,
                "destination_chain": op.destination_chain,
                "amount": unit::format_amount(&op.source_amount, &op.asset),
                "amount_raw": op.source_amount,
                "state": op.state,
                "source_tx": op.source_tx_hash,
                "destination_tx": op.destination_tx_hash,
                "confirmations": op.source_tx_confirmations,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", "━".repeat(60).dimmed());
        println!(
            "  {} for {}",
            "Bridge Operations".cyan().bold(),
            cfg.address.dimmed()
        );
        println!("{}", "━".repeat(60).dimmed());

        if ops.operations.is_empty() {
            println!();
            println!("  No bridge operations found.");
            println!();
            return Ok(());
        }

        for op in &ops.operations {
            let direction = if op.destination_chain == "hyperliquid" {
                "DEPOSIT".green()
            } else {
                "WITHDRAW".red()
            };
            let amount = unit::format_amount(&op.source_amount, &op.asset);
            let state_color = match op.state.as_str() {
                "done" => op.state.green(),
                s if s.contains("wait") || s.contains("Wait") => op.state.yellow(),
                _ => op.state.normal(),
            };

            println!();
            println!(
                "  {} {} {} → {}",
                direction,
                amount.cyan(),
                op.source_chain,
                op.destination_chain
            );
            println!(
                "    {} {}  {} {}",
                "State:".dimmed(),
                state_color,
                "Created:".dimmed(),
                op.op_created_at.dimmed()
            );
            if !op.source_tx_hash.is_empty() {
                let hash = if op.source_tx_hash.len() > 20 {
                    format!("{}…", &op.source_tx_hash[..20])
                } else {
                    op.source_tx_hash.clone()
                };
                println!("    {} {}", "Src TX: ".dimmed(), hash);
            }
            if let Some(ref dtx) = op.destination_tx_hash {
                if !dtx.is_empty() {
                    let hash = if dtx.len() > 20 {
                        format!("{}…", &dtx[..20])
                    } else {
                        dtx.clone()
                    };
                    println!("    {} {}", "Dst TX: ".dimmed(), hash);
                }
            }
        }
        println!();
    }

    Ok(())
}
