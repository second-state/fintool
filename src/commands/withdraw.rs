//! `fintool withdraw <amount> <asset> --to <addr>` — withdraw from exchange
//!
//! For Hyperliquid (default):
//!   - USDC: HL SDK withdraw_from_bridge (HL → Arbitrum, ~3-4 min)
//!   - ETH/BTC/SOL: Unit bridge (HL → native chain)
//!
//! For Binance (--exchange binance):
//!   - POST /sapi/v1/capital/withdraw/apply
//!
//! For Coinbase (--exchange coinbase):
//!   - POST /v2/accounts/{id}/transactions (type=send)

use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::binance;
use crate::coinbase;
use crate::config;
use crate::unit;

pub async fn run(
    amount: &str,
    asset: &str,
    to: Option<&str>,
    network: Option<&str>,
    exchange: &str,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    match exchange.to_lowercase().as_str() {
        "binance" => withdraw_binance(amount, asset, to, network, dry_run, json_out).await,
        "coinbase" => withdraw_coinbase(amount, asset, to, network, dry_run, json_out).await,
        "hyperliquid" | "auto" => {
            let asset_lower = asset.to_lowercase();
            if asset_lower == "usdc" {
                withdraw_usdc_hl(amount, to, dry_run, json_out).await
            } else {
                withdraw_unit(amount, asset, to, dry_run, json_out).await
            }
        }
        other => bail!(
            "Unsupported exchange '{}'. Use: hyperliquid, binance, coinbase",
            other
        ),
    }
}

// ── HL USDC withdrawal (Bridge2) ─────────────────────────────────────

async fn withdraw_usdc_hl(
    amount: &str,
    to: Option<&str>,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let destination = to.unwrap_or(&cfg.address);

    if dry_run {
        if json_out {
            let out = json!({
                "action": "withdraw_quote",
                "exchange": "hyperliquid",
                "asset": "USDC",
                "amount": amount,
                "destination_chain": "arbitrum",
                "destination_address": destination,
                "method": "bridge2_withdraw3",
                "estimated_time": "3-4 minutes",
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("{}", "━".repeat(50).dimmed());
            println!(
                "  {} {} USDC → Arbitrum  {}",
                "Withdraw".red().bold(),
                amount,
                "(dry run)".dimmed()
            );
            println!("{}", "━".repeat(50).dimmed());
            println!();
            println!("  {} {}", "Destination:".dimmed(), destination.cyan());
            println!("  {} ~3-4 minutes", "Est. time:  ".dimmed());
            println!();
        }
        return Ok(());
    }

    // Execute withdrawal via HL SDK
    eprintln!("Withdrawing {} USDC from Hyperliquid to {}...", amount, destination);

    use ethers::signers::LocalWallet;
    use hyperliquid_rust_sdk::{BaseUrl, ExchangeClient};

    let wallet: LocalWallet = cfg
        .private_key
        .parse()
        .context("Invalid private key")?;

    let base_url = if cfg.testnet {
        BaseUrl::Testnet
    } else {
        BaseUrl::Mainnet
    };

    let exchange_client = ExchangeClient::new(None, wallet, Some(base_url), None, None).await?;

    let result = exchange_client
        .withdraw_from_bridge(amount, destination, None)
        .await
        .context("Failed to withdraw from Hyperliquid")?;

    if json_out {
        let out = json!({
            "action": "withdraw",
            "exchange": "hyperliquid",
            "status": "submitted",
            "asset": "USDC",
            "amount": amount,
            "destination_chain": "arbitrum",
            "destination_address": destination,
            "result": format!("{:?}", result),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!();
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} USDC withdrawal submitted",
            "✅".green(),
            amount
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!("  {} {}", "Destination:".dimmed(), destination.cyan());
        println!("  {} Arbitrum", "Chain:      ".dimmed());
        println!("  {} ~3-4 minutes", "Est. time:  ".dimmed());
        println!();
    }

    Ok(())
}

// ── Unit bridge withdrawal (ETH/BTC/SOL) ─────────────────────────────

async fn withdraw_unit(
    amount: &str,
    asset: &str,
    to: Option<&str>,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    let asset_lower = asset.to_lowercase();

    if !unit::is_supported(&asset_lower) {
        bail!(
            "Unsupported asset '{}'. Supported: ETH, BTC, SOL, USDC",
            asset
        );
    }

    let cfg = config::load_hl_config()?;
    let chain = unit::native_chain(&asset_lower).unwrap();
    let min = unit::minimum_amount(&asset_lower).unwrap_or("unknown");

    let dst_addr = match to {
        Some(addr) => addr.to_string(),
        None => {
            if asset_lower == "btc" {
                bail!(
                    "BTC withdrawals require --to <bitcoin_address>.\n\
                     Usage: fintool withdraw 0.01 BTC --to bc1q..."
                );
            }
            cfg.address.clone()
        }
    };

    // Generate withdrawal address: hyperliquid → native chain
    let resp =
        unit::generate_address("hyperliquid", chain, &asset_lower, &dst_addr, cfg.testnet)
            .await?;

    let fees = unit::estimate_fees(cfg.testnet).await.ok();

    if dry_run {
        if json_out {
            let mut out = json!({
                "action": "withdraw_quote",
                "exchange": "hyperliquid",
                "asset": asset.to_uppercase(),
                "amount": amount,
                "destination_chain": chain,
                "destination_address": dst_addr,
                "unit_withdraw_address": resp.address,
                "minimum": min,
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
                "  {} {} u{} → {}  {}",
                "Withdraw".red().bold(),
                amount,
                asset.to_uppercase().cyan(),
                chain.yellow(),
                "(dry run)".dimmed()
            );
            println!("{}", "━".repeat(50).dimmed());
            println!();
            println!("  {} {}", "Dest chain:  ".dimmed(), chain.yellow());
            println!("  {} {}", "Dest address:".dimmed(), dst_addr.cyan());
            println!("  {} {}", "Minimum:     ".dimmed(), min);
            println!();
            println!(
                "  {} Transfer u{} on HL to: {}",
                "→".red().bold(),
                asset.to_uppercase(),
                resp.address
            );
            println!();
        }
        return Ok(());
    }

    // Execute: transfer uAsset on HL to the Unit withdrawal address
    eprintln!(
        "Withdrawing {} u{} from Hyperliquid to {} on {}...",
        amount,
        asset.to_uppercase(),
        dst_addr,
        chain
    );

    // Use HL SDK spot transfer to send uAsset to the Unit withdrawal address
    use ethers::signers::LocalWallet;
    use hyperliquid_rust_sdk::{BaseUrl, ExchangeClient};

    let wallet: LocalWallet = cfg.private_key.parse().context("Invalid private key")?;
    let base_url = if cfg.testnet {
        BaseUrl::Testnet
    } else {
        BaseUrl::Mainnet
    };

    let exchange_client = ExchangeClient::new(None, wallet, Some(base_url), None, None).await?;

    // SpotSend: transfer uAsset to Unit withdrawal address on HL
    let result = exchange_client
        .spot_transfer(amount, &resp.address, &asset_lower, None)
        .await
        .context("Failed to transfer to Unit withdrawal address")?;

    if json_out {
        let out = json!({
            "action": "withdraw",
            "exchange": "hyperliquid",
            "status": "submitted",
            "asset": asset.to_uppercase(),
            "amount": amount,
            "destination_chain": chain,
            "destination_address": dst_addr,
            "unit_withdraw_address": resp.address,
            "result": format!("{:?}", result),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!();
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} u{} withdrawal submitted",
            "✅".green(),
            amount,
            asset.to_uppercase()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!("  {} {}", "Dest chain:  ".dimmed(), chain.yellow());
        println!("  {} {}", "Dest address:".dimmed(), dst_addr.cyan());
        println!("  {} fintool bridge-status", "Track:      ".dimmed());
        println!();
    }

    Ok(())
}

// ── Binance withdrawal ───────────────────────────────────────────────

async fn withdraw_binance(
    amount: &str,
    asset: &str,
    to: Option<&str>,
    network: Option<&str>,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    let to = to.ok_or_else(|| {
        anyhow::anyhow!(
            "Binance withdrawals require --to <address>.\n\
             Usage: fintool withdraw 100 USDC --to 0x... --exchange binance"
        )
    })?;

    let (api_key, api_secret) = config::binance_credentials()
        .ok_or_else(|| anyhow::anyhow!("Binance API keys not configured in ~/.fintool/config.toml"))?;

    // Map chain names to Binance network codes
    let binance_network: Option<String> = network.map(|n| {
        match n.to_lowercase().as_str() {
            "ethereum" | "eth" | "mainnet" | "erc20" => "ETH".to_string(),
            "base" => "BASE".to_string(),
            "arbitrum" | "arb" => "ARBITRUM".to_string(),
            "solana" | "sol" => "SOL".to_string(),
            "bitcoin" | "btc" => "BTC".to_string(),
            "bsc" | "bnb" => "BSC".to_string(),
            "polygon" | "matic" => "MATIC".to_string(),
            "optimism" | "op" => "OPTIMISM".to_string(),
            "avalanche" | "avax" => "AVAXC".to_string(),
            _ => n.to_uppercase(),
        }
    });

    if dry_run {
        if json_out {
            let mut out = json!({
                "action": "withdraw_quote",
                "exchange": "binance",
                "asset": asset.to_uppercase(),
                "amount": amount,
                "destination_address": to,
            });
            if let Some(ref net) = binance_network {
                out["network"] = json!(net);
            }
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("{}", "━".repeat(50).dimmed());
            println!(
                "  {} {} {} from Binance  {}",
                "Withdraw".red().bold(),
                amount,
                asset.to_uppercase().cyan(),
                "(dry run)".dimmed()
            );
            println!("{}", "━".repeat(50).dimmed());
            println!();
            println!("  {} {}", "To:      ".dimmed(), to.cyan());
            if let Some(ref net) = binance_network {
                println!("  {} {}", "Network: ".dimmed(), net.yellow());
            }
            println!();
        }
        return Ok(());
    }

    eprintln!(
        "Withdrawing {} {} from Binance to {}...",
        amount,
        asset.to_uppercase(),
        to
    );

    let client = reqwest::Client::new();
    let resp = binance::withdraw(
        &client,
        &api_key,
        &api_secret,
        asset,
        to,
        amount,
        binance_network.as_deref(),
    )
    .await?;

    let withdraw_id = resp["id"].as_str().unwrap_or("unknown");

    if json_out {
        let out = json!({
            "action": "withdraw",
            "exchange": "binance",
            "status": "submitted",
            "asset": asset.to_uppercase(),
            "amount": amount,
            "destination_address": to,
            "withdraw_id": withdraw_id,
            "network": binance_network,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!();
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} {} withdrawal submitted",
            "✅".green(),
            amount,
            asset.to_uppercase()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!("  {} {}", "To:          ".dimmed(), to.cyan());
        println!("  {} {}", "Withdraw ID: ".dimmed(), withdraw_id);
        if let Some(ref net) = binance_network {
            println!("  {} {}", "Network:     ".dimmed(), net.yellow());
        }
        println!();
    }

    Ok(())
}

// ── Coinbase withdrawal (send) ───────────────────────────────────────

async fn withdraw_coinbase(
    amount: &str,
    asset: &str,
    to: Option<&str>,
    network: Option<&str>,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    let to = to.ok_or_else(|| {
        anyhow::anyhow!(
            "Coinbase withdrawals require --to <address>.\n\
             Usage: fintool withdraw 100 USDC --to 0x... --exchange coinbase"
        )
    })?;

    let (api_key, api_secret) = config::coinbase_credentials()
        .ok_or_else(|| anyhow::anyhow!("Coinbase API keys not configured in ~/.fintool/config.toml"))?;

    let client = reqwest::Client::new();

    // Find account for this asset
    let accounts = coinbase::list_accounts_raw(&client, &api_key, &api_secret).await?;

    let account_id = accounts["accounts"]
        .as_array()
        .and_then(|accs: &Vec<serde_json::Value>| {
            accs.iter().find(|a: &&serde_json::Value| {
                a["currency"]
                    .as_str()
                    .map(|c: &str| c.eq_ignore_ascii_case(asset))
                    .unwrap_or(false)
            })
        })
        .and_then(|a: &serde_json::Value| a["uuid"].as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No Coinbase account found for {}. Check your API permissions.",
                asset.to_uppercase()
            )
        })?
        .to_string();

    if dry_run {
        if json_out {
            let mut out = json!({
                "action": "withdraw_quote",
                "exchange": "coinbase",
                "asset": asset.to_uppercase(),
                "amount": amount,
                "destination_address": to,
                "account_id": account_id,
            });
            if let Some(net) = network {
                out["network"] = json!(net);
            }
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("{}", "━".repeat(50).dimmed());
            println!(
                "  {} {} {} from Coinbase  {}",
                "Withdraw".red().bold(),
                amount,
                asset.to_uppercase().cyan(),
                "(dry run)".dimmed()
            );
            println!("{}", "━".repeat(50).dimmed());
            println!();
            println!("  {} {}", "To:      ".dimmed(), to.cyan());
            if let Some(net) = network {
                println!("  {} {}", "Network: ".dimmed(), net.yellow());
            }
            println!();
        }
        return Ok(());
    }

    eprintln!(
        "Withdrawing {} {} from Coinbase to {}...",
        amount,
        asset.to_uppercase(),
        to
    );

    let resp = coinbase::send_crypto(
        &client,
        &api_key,
        &api_secret,
        &account_id,
        to,
        amount,
        asset,
        network,
    )
    .await?;

    let tx_id = resp["data"]["id"].as_str().unwrap_or("unknown");

    if json_out {
        let out = json!({
            "action": "withdraw",
            "exchange": "coinbase",
            "status": "submitted",
            "asset": asset.to_uppercase(),
            "amount": amount,
            "destination_address": to,
            "transaction_id": tx_id,
            "network": network,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!();
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} {} withdrawal submitted",
            "✅".green(),
            amount,
            asset.to_uppercase()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!("  {} {}", "To:      ".dimmed(), to.cyan());
        println!("  {} {}", "TX ID:   ".dimmed(), tx_id);
        println!();
    }

    Ok(())
}
