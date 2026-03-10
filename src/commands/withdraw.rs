//! `fintool withdraw <asset> --amount <amt> --to <dest>` — withdraw from exchange
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
//!
//! For Polymarket (--exchange polymarket):
//!   - Bridge API: generates withdrawal address, user sends USDC.e on Polygon

use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::binance;
use crate::bridge::{self, DestChain};
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
        "polymarket" => withdraw_polymarket(amount, asset, to, network, json_out).await,
        "binance" => withdraw_binance(amount, asset, to, network, dry_run, json_out).await,
        "coinbase" => withdraw_coinbase(amount, asset, to, network, dry_run, json_out).await,
        "hyperliquid" | "auto" => {
            let asset_lower = asset.to_lowercase();
            if asset_lower == "usdc" {
                withdraw_usdc_hl(amount, to, network, dry_run, json_out).await
            } else {
                withdraw_unit(amount, asset, to, dry_run, json_out).await
            }
        }
        other => bail!(
            "Unsupported exchange '{}'. Use: hyperliquid, binance, coinbase, polymarket",
            other
        ),
    }
}

// ── HL USDC withdrawal (Bridge2) ─────────────────────────────────────

async fn withdraw_usdc_hl(
    amount: &str,
    to: Option<&str>,
    network: Option<&str>,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    let cfg = config::load_hl_config()?;
    let destination = to.unwrap_or(&cfg.address).to_string();

    // If network is ethereum or base, chain: HL → Arbitrum → target via Across
    let dest_chain: Option<DestChain> = match network {
        Some(n) => match n.to_lowercase().as_str() {
            "arbitrum" | "arb" | "" => None, // default, no extra bridge
            other => Some(other.parse()?),
        },
        None => None,
    };

    if let Some(dest) = dest_chain {
        return withdraw_usdc_hl_bridged(amount, &destination, dest, dry_run, json_out).await;
    }

    // Simple case: HL → Arbitrum only
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

    eprintln!(
        "Withdrawing {} USDC from Hyperliquid to Arbitrum...",
        amount
    );

    use ethers::signers::LocalWallet;
    use hyperliquid_rust_sdk::{BaseUrl, ExchangeClient};

    let wallet: LocalWallet = cfg.private_key.parse().context("Invalid private key")?;
    let base_url = if cfg.testnet {
        BaseUrl::Testnet
    } else {
        BaseUrl::Mainnet
    };

    let exchange_client = ExchangeClient::new(None, wallet, Some(base_url), None, None).await?;
    let result = exchange_client
        .withdraw_from_bridge(amount, &destination, None)
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
        println!("  {} {} USDC withdrawal submitted", "✅".green(), amount);
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!("  {} {}", "Destination:".dimmed(), destination.cyan());
        println!("  {} Arbitrum", "Chain:      ".dimmed());
        println!("  {} ~3-4 minutes", "Est. time:  ".dimmed());
        println!();
    }

    Ok(())
}

/// Chained withdrawal: HL → Arbitrum (Bridge2) → Ethereum/Base (Across)
async fn withdraw_usdc_hl_bridged(
    amount: &str,
    destination: &str,
    dest_chain: DestChain,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    use ethers::prelude::*;

    let cfg = config::load_hl_config()?;

    // Get Across quote for the Arbitrum → destination leg
    eprintln!(
        "Fetching Across bridge quote (Arbitrum → {})...",
        dest_chain.name()
    );
    let quote = bridge::get_across_quote_reverse(dest_chain, amount, &cfg.address).await?;

    let output_amount = quote
        .expected_output_amount
        .as_deref()
        .unwrap_or(&quote.input_amount);
    let fill_time = quote.expected_fill_time.unwrap_or(0);
    let needs_approval = quote.approval_txns.as_ref().is_some_and(|a| !a.is_empty());

    if dry_run {
        if json_out {
            let mut out = json!({
                "action": "withdraw_quote",
                "exchange": "hyperliquid",
                "asset": "USDC",
                "amount": amount,
                "route": format!("hyperliquid → arbitrum → {}", dest_chain.name()),
                "destination_chain": dest_chain.name(),
                "destination_address": destination,
                "amount_out": bridge::format_usdc(output_amount),
                "estimated_fill_time_seconds": fill_time,
                "needs_approval": needs_approval,
            });
            if let Some(ref fees) = quote.fees {
                out["across_fees"] = fees.clone();
            }
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("{}", "━".repeat(55).dimmed());
            println!(
                "  {} {} USDC → {}  {}",
                "Withdraw".red().bold(),
                amount,
                dest_chain.name().yellow(),
                "(dry run)".dimmed()
            );
            println!("{}", "━".repeat(55).dimmed());
            println!();
            println!(
                "  {} HL → Arbitrum → {}",
                "Route:      ".dimmed(),
                dest_chain.name()
            );
            println!("  {} {}", "Amount in:  ".dimmed(), amount);
            println!(
                "  {} {}",
                "Amount out: ".dimmed(),
                bridge::format_usdc(output_amount).green()
            );
            println!("  {} ~{}s (Across leg)", "Fill time:  ".dimmed(), fill_time);
            println!("  {} {}", "Destination:".dimmed(), destination.cyan());
            println!();
        }
        return Ok(());
    }

    // Step 1: Withdraw USDC from HL to Arbitrum
    eprintln!("Step 1: Withdrawing {} USDC from HL to Arbitrum...", amount);

    let wallet: ethers::signers::LocalWallet =
        cfg.private_key.parse().context("Invalid private key")?;
    let base_url = if cfg.testnet {
        hyperliquid_rust_sdk::BaseUrl::Testnet
    } else {
        hyperliquid_rust_sdk::BaseUrl::Mainnet
    };

    let exchange_client =
        hyperliquid_rust_sdk::ExchangeClient::new(None, wallet, Some(base_url), None, None).await?;
    exchange_client
        .withdraw_from_bridge(amount, &cfg.address, None)
        .await
        .context("Failed to withdraw from Hyperliquid")?;

    eprintln!("  ✅ HL withdrawal submitted. Waiting for USDC on Arbitrum (~4 min)...");

    // Step 2: Poll Arbitrum USDC balance until funds arrive
    // HL Bridge2 takes ~3-4 minutes
    let arb_provider = Provider::<Http>::try_from(bridge::RPC_ARBITRUM)
        .context("Failed to connect to Arbitrum RPC")?;
    let arb_wallet: LocalWallet = cfg
        .private_key
        .parse::<LocalWallet>()
        .context("Invalid private key")?
        .with_chain_id(bridge::ARBITRUM_CHAIN_ID);
    let arb_client = std::sync::Arc::new(SignerMiddleware::new(arb_provider, arb_wallet));

    let usdc_addr: Address = bridge::USDC_ARBITRUM
        .parse()
        .context("Invalid USDC address")?;
    let user_addr: Address = cfg.address.parse().context("Invalid user address")?;
    // balanceOf(address) selector = 0x70a08231
    let balance_calldata = {
        let mut data = vec![0x70, 0xa0, 0x82, 0x31];
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(user_addr.as_bytes());
        ethers::types::Bytes::from(data)
    };

    // Record initial Arbitrum USDC balance before HL withdrawal
    let mut arb_usdc_balance = U256::zero();
    {
        let call_tx = ethers::types::TransactionRequest::new()
            .to(usdc_addr)
            .data(balance_calldata.clone());
        if let Ok(result) = arb_client.call(&call_tx.into(), None).await {
            if result.len() >= 32 {
                arb_usdc_balance = U256::from_big_endian(&result[..32]);
            }
        }
    }
    let initial_arb_balance = arb_usdc_balance;

    // HL Bridge2 charges a ~$1 fee, so we check for any increase over
    // the initial balance rather than the full requested amount.
    for attempt in 1..=20 {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        let call_tx = ethers::types::TransactionRequest::new()
            .to(usdc_addr)
            .data(balance_calldata.clone());
        if let Ok(result) = arb_client.call(&call_tx.into(), None).await {
            if result.len() >= 32 {
                arb_usdc_balance = U256::from_big_endian(&result[..32]);
            }
        }
        let bal_f = arb_usdc_balance.as_u128() as f64 / 1e6;
        eprintln!(
            "  Checking Arbitrum USDC... ${:.2} (attempt {}/20)",
            bal_f, attempt
        );
        if arb_usdc_balance > initial_arb_balance {
            break;
        }
    }

    if arb_usdc_balance <= initial_arb_balance {
        bail!(
            "USDC did not arrive on Arbitrum after 10 minutes. Balance: {}",
            arb_usdc_balance,
        );
    }

    let arrived_amount = arb_usdc_balance - initial_arb_balance;
    let arrived_usdc = format!(
        "{}.{:06}",
        arrived_amount / 1_000_000,
        arrived_amount % 1_000_000
    );
    eprintln!(
        "  ✅ USDC arrived on Arbitrum! (${} after Bridge2 fee)",
        arrived_amount.as_u128() as f64 / 1e6
    );

    // Step 3: Bridge USDC from Arbitrum → destination via Across
    eprintln!(
        "Step 2: Bridging USDC from Arbitrum → {} via Across...",
        dest_chain.name()
    );

    // Re-fetch Across quote using the actual arrived amount (after Bridge2 fee).
    // The original quote used the pre-fee amount and has stale calldata.
    eprintln!("  Refreshing Across bridge quote...");
    let quote = bridge::get_across_quote_reverse(dest_chain, &arrived_usdc, &cfg.address).await?;
    let output_amount = quote
        .expected_output_amount
        .as_deref()
        .unwrap_or(&quote.input_amount);

    // Get gas price with 50% buffer to avoid "max fee less than base fee" errors
    let arb_gas_price = arb_client
        .get_gas_price()
        .await
        .context("Failed to get Arbitrum gas price")?;
    let arb_gas_price_buffered = arb_gas_price * 150 / 100;

    // Approval txns (if needed)
    if let Some(ref approval_txns) = quote.approval_txns {
        for (i, atx) in approval_txns.iter().enumerate() {
            eprintln!("  Sending approval tx {}/{}...", i + 1, approval_txns.len());
            let tx = TransactionRequest::new()
                .to(atx.to.parse::<Address>().context("Invalid address")?)
                .data(
                    hex::decode(atx.data.strip_prefix("0x").unwrap_or(&atx.data))
                        .context("Invalid data")?,
                )
                .chain_id(bridge::ARBITRUM_CHAIN_ID)
                .gas_price(arb_gas_price_buffered);

            let pending = arb_client
                .send_transaction(tx, None)
                .await
                .context("Failed to send approval tx")?;
            let receipt = pending
                .await
                .context("Approval tx failed")?
                .ok_or_else(|| anyhow::anyhow!("Approval tx dropped"))?;
            eprintln!("  ✅ Approval confirmed: {:?}", receipt.transaction_hash);
        }
    }

    // Bridge tx
    eprintln!("  Sending Across bridge tx...");
    let bridge_value = quote
        .swap_tx
        .value
        .as_ref()
        .and_then(|v| U256::from_dec_str(v).ok())
        .unwrap_or_default();

    let bridge_tx = TransactionRequest::new()
        .to(quote
            .swap_tx
            .to
            .parse::<Address>()
            .context("Invalid bridge address")?)
        .data(
            hex::decode(
                quote
                    .swap_tx
                    .data
                    .strip_prefix("0x")
                    .unwrap_or(&quote.swap_tx.data),
            )
            .context("Invalid bridge data")?,
        )
        .value(bridge_value)
        .chain_id(bridge::ARBITRUM_CHAIN_ID)
        .gas_price(arb_gas_price_buffered);

    let pending = arb_client
        .send_transaction(bridge_tx, None)
        .await
        .context("Failed to send bridge tx")?;
    let bridge_receipt = pending
        .await
        .context("Bridge tx failed")?
        .ok_or_else(|| anyhow::anyhow!("Bridge tx dropped"))?;

    let bridge_tx_hash = format!("{:?}", bridge_receipt.transaction_hash);
    eprintln!("  ✅ Bridge tx confirmed: {}", bridge_tx_hash);

    if json_out {
        let out = json!({
            "action": "withdraw",
            "exchange": "hyperliquid",
            "status": "completed",
            "asset": "USDC",
            "amount": amount,
            "amount_out": bridge::format_usdc(output_amount),
            "route": format!("hyperliquid → arbitrum → {}", dest_chain.name()),
            "destination_chain": dest_chain.name(),
            "destination_address": destination,
            "bridge_tx": bridge_tx_hash,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!();
        println!("{}", "━".repeat(55).dimmed());
        println!(
            "  {} {} USDC → {}",
            "✅".green(),
            bridge::format_usdc(output_amount).green().bold(),
            dest_chain.name()
        );
        println!("{}", "━".repeat(55).dimmed());
        println!();
        println!(
            "  {} HL → Arbitrum → {}",
            "Route:      ".dimmed(),
            dest_chain.name()
        );
        println!("  {} {}", "Destination:".dimmed(), destination.cyan());
        println!("  {} {}", "Bridge TX:  ".dimmed(), bridge_tx_hash.cyan());
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
                     Usage: fintool withdraw BTC --amount 0.01 --to bc1q..."
                );
            }
            cfg.address.clone()
        }
    };

    // Generate withdrawal address: hyperliquid → native chain
    let resp =
        unit::generate_address("hyperliquid", chain, &asset_lower, &dst_addr, cfg.testnet).await?;

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
             Usage: fintool withdraw USDC --amount 100 --to 0x... --exchange binance"
        )
    })?;

    let (api_key, api_secret) = config::binance_credentials().ok_or_else(|| {
        anyhow::anyhow!("Binance API keys not configured in ~/.fintool/config.toml")
    })?;

    // Map chain names to Binance network codes
    let binance_network: Option<String> = network.map(|n| match n.to_lowercase().as_str() {
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
             Usage: fintool withdraw USDC --amount 100 --to 0x... --exchange coinbase"
        )
    })?;

    let (api_key, api_secret) = config::coinbase_credentials().ok_or_else(|| {
        anyhow::anyhow!("Coinbase API keys not configured in ~/.fintool/config.toml")
    })?;

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

// ── Polymarket withdrawal (Bridge API) ────────────────────────────

/// Resolve destination chain info from --to / --network flags.
/// Returns (chain_id, usdc_address, chain_name).
fn resolve_polymarket_dest(
    to: Option<&str>,
    network: Option<&str>,
) -> Result<(u64, &'static str, &'static str)> {
    let chain_name = network
        .or(to)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Polymarket withdraw requires --to <chain>.\n\
                 Usage: fintool withdraw USDC --amount 10 --to base --exchange polymarket\n\
                 Supported chains: base, ethereum, arbitrum"
            )
        })?
        .to_lowercase();

    match chain_name.as_str() {
        "base" => Ok((8453, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "Base")),
        "ethereum" | "eth" | "mainnet" => {
            Ok((1, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "Ethereum"))
        }
        "arbitrum" | "arb" => Ok((
            42161,
            "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
            "Arbitrum",
        )),
        other => bail!(
            "Unsupported destination chain '{}'. Supported: base, ethereum, arbitrum",
            other
        ),
    }
}

async fn withdraw_polymarket(
    amount: &str,
    asset: &str,
    to: Option<&str>,
    network: Option<&str>,
    json_out: bool,
) -> Result<()> {
    let asset_upper = asset.to_uppercase();
    if asset_upper != "USDC" {
        bail!("Polymarket only supports USDC withdrawals. Got '{}'", asset);
    }

    let (chain_id, usdc_address, chain_name) = resolve_polymarket_dest(to, network)?;

    let address = crate::polymarket::get_polymarket_address()?;
    let client = crate::polymarket::create_bridge_client();

    use std::str::FromStr;
    let addr = alloy::primitives::Address::from_str(&address)
        .context("Invalid Polymarket wallet address")?;

    use polymarket_client_sdk::bridge::types::WithdrawRequest;
    let req = WithdrawRequest::builder()
        .address(addr)
        .to_chain_id(chain_id)
        .to_token_address(usdc_address)
        .recipient_addr(&address)
        .build();

    let resp = client
        .withdraw(&req)
        .await
        .context("Failed to get Polymarket withdrawal address")?;

    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "withdraw",
                "exchange": "polymarket",
                "asset": "USDC",
                "amount": amount,
                "destination_chain": chain_name,
                "destination_address": address,
                "withdrawal_address_evm": format!("{}", resp.address.evm),
                "note": resp.note,
            }))?
        );
    } else {
        println!("{}", "━".repeat(55).dimmed());
        println!(
            "  {} {} USDC → {}  [Polymarket]",
            "Withdraw".red().bold(),
            amount,
            chain_name.yellow(),
        );
        println!("{}", "━".repeat(55).dimmed());
        println!();
        println!("  {} {}", "Destination:".dimmed(), chain_name.yellow());
        println!("  {} {}", "Recipient:  ".dimmed(), address.cyan());
        println!();
        println!(
            "  {} Send {} USDC.e on Polygon to:",
            "→".red().bold(),
            amount
        );
        println!();
        println!("    {}", format!("{}", resp.address.evm).green().bold());
        println!();
        println!("  {} {}", "Note:".dimmed(), resp.note);
        println!();
    }

    Ok(())
}
