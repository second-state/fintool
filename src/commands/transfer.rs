/// Transfer assets between perp, spot, and HIP-3 dex accounts on Hyperliquid.
use anyhow::{Context, Result};

use crate::{config, signing};

pub async fn run(asset: &str, amount: &str, from: &str, to: &str, json_output: bool) -> Result<()> {
    config::load_hl_config()
        .context("Hyperliquid wallet not configured. Transfer requires Hyperliquid.")?;
    let amount_f: f64 = amount
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount))?;

    let from_lower = from.to_lowercase();
    let to_lower = to.to_lowercase();

    // Determine transfer type based on from/to
    if (from_lower == "perp" && to_lower == "spot") || (from_lower == "spot" && to_lower == "perp")
    {
        // Perp ↔ spot transfer (USDC)
        let to_perp = to_lower == "perp";
        let dir_label = if to_perp {
            "spot → perp"
        } else {
            "perp → spot"
        };
        signing::class_transfer(amount_f, to_perp).await?;
        if json_output {
            println!(
                "{}",
                serde_json::json!({
                    "action": "transfer",
                    "asset": asset,
                    "amount": amount,
                    "from": from,
                    "to": to,
                    "status": "ok",
                })
            );
        } else {
            println!("  Transferred ${} {} ({})", amount, asset, dir_label);
        }
    } else if to_lower == "spot" && from_lower != "perp" {
        // From dex → spot
        let dex_name = &from_lower;
        let (collateral_token, token_name) = signing::get_dex_collateral_token(dex_name).await?;
        let dir_label = format!("{} dex → spot", dex_name);
        signing::send_asset(amount_f, dex_name, "spot", &collateral_token).await?;
        if json_output {
            println!(
                "{}",
                serde_json::json!({
                    "action": "transfer",
                    "asset": asset,
                    "amount": amount,
                    "from": from,
                    "to": to,
                    "token": token_name,
                    "status": "ok",
                })
            );
        } else {
            println!("  Transferred ${} {} ({})", amount, token_name, dir_label);
        }
    } else if from_lower == "spot" && to_lower != "perp" {
        // Spot → dex
        let dex_name = &to_lower;
        let (collateral_token, token_name) = signing::get_dex_collateral_token(dex_name).await?;
        let dir_label = format!("spot → {} dex", dex_name);
        signing::send_asset(amount_f, "spot", dex_name, &collateral_token).await?;
        if json_output {
            println!(
                "{}",
                serde_json::json!({
                    "action": "transfer",
                    "asset": asset,
                    "amount": amount,
                    "from": from,
                    "to": to,
                    "token": token_name,
                    "status": "ok",
                })
            );
        } else {
            println!("  Transferred ${} {} ({})", amount, token_name, dir_label);
        }
    } else {
        anyhow::bail!(
            "Invalid transfer: --from {} --to {}. One side must be 'spot'. \
             Use: --from spot --to perp, --from perp --to spot, \
             --from spot --to <dex>, or --from <dex> --to spot",
            from,
            to
        );
    }
    Ok(())
}
