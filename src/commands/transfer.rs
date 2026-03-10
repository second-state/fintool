/// Transfer assets between perp, spot, and HIP-3 dex accounts.
use anyhow::{Context, Result};

use crate::{binance, config, signing};

pub async fn run(
    asset: &str,
    amount: &str,
    from: &str,
    to: &str,
    exchange: &str,
    json_output: bool,
) -> Result<()> {
    if exchange == "binance" {
        return run_binance(asset, amount, from, to, json_output).await;
    }

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

/// Binance universal transfer between spot and futures wallets
async fn run_binance(
    asset: &str,
    amount: &str,
    from: &str,
    to: &str,
    json_output: bool,
) -> Result<()> {
    let (api_key, api_secret) = config::binance_credentials()
        .ok_or_else(|| anyhow::anyhow!("Binance API credentials not configured"))?;

    let from_lower = from.to_lowercase();
    let to_lower = to.to_lowercase();

    let transfer_type = match (from_lower.as_str(), to_lower.as_str()) {
        ("spot", "futures") | ("spot", "perp") | ("main", "umfuture") => "MAIN_UMFUTURE",
        ("futures", "spot") | ("perp", "spot") | ("umfuture", "main") => "UMFUTURE_MAIN",
        _ => {
            anyhow::bail!(
                "Invalid Binance transfer: --from {} --to {}. \
                 Use: --from spot --to futures, or --from futures --to spot",
                from,
                to
            );
        }
    };

    let dir_label = if transfer_type == "MAIN_UMFUTURE" {
        "spot → futures"
    } else {
        "futures → spot"
    };

    let client = reqwest::Client::new();
    let result =
        binance::universal_transfer(&client, &api_key, &api_secret, asset, amount, transfer_type)
            .await?;

    let txn_id = result.get("tranId").and_then(|v| v.as_u64()).unwrap_or(0);

    if json_output {
        println!(
            "{}",
            serde_json::json!({
                "action": "transfer",
                "exchange": "binance",
                "asset": asset.to_uppercase(),
                "amount": amount,
                "from": from,
                "to": to,
                "transfer_type": transfer_type,
                "txn_id": txn_id,
                "status": "ok",
            })
        );
    } else {
        println!(
            "  Transferred {} {} on Binance ({})",
            amount,
            asset.to_uppercase(),
            dir_label
        );
        if txn_id > 0 {
            println!("  Transaction ID: {}", txn_id);
        }
    }

    Ok(())
}
