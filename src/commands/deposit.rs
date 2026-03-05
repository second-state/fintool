//! `fintool deposit <asset>` — deposit to exchange
//!
//! For Hyperliquid (default):
//!   - ETH/BTC/SOL: generates permanent Unit deposit address
//!   - USDC --amount X --from ethereum|base: bridges via Across → Arbitrum → HL Bridge2
//!
//! For Binance (--exchange binance):
//!   - Any asset: fetches Binance deposit address via API
//!
//! For Coinbase (--exchange coinbase):
//!   - Any asset: creates Coinbase deposit address via API

use anyhow::{bail, Context, Result};
use colored::Colorize;
use ethers::prelude::*;
use serde_json::json;

use crate::binance;
use crate::bridge::{self, SourceChain};
use crate::coinbase;
use crate::config;
use crate::unit;

pub async fn run(
    asset: &str,
    amount: Option<&str>,
    from: Option<&str>,
    exchange: &str,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    match exchange.to_lowercase().as_str() {
        "binance" => deposit_binance(asset, from, json_out).await,
        "coinbase" => deposit_coinbase(asset, json_out).await,
        "hyperliquid" | "auto" => {
            let asset_lower = asset.to_lowercase();
            if asset_lower == "usdc" {
                deposit_usdc_hl(amount, from, dry_run, json_out).await
            } else {
                deposit_unit(asset, json_out).await
            }
        }
        other => bail!(
            "Unsupported exchange '{}'. Use: hyperliquid, binance, coinbase",
            other
        ),
    }
}

// ── Unit bridge (ETH/BTC/SOL → HL) ──────────────────────────────────

async fn deposit_unit(asset: &str, json_out: bool) -> Result<()> {
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

    let resp = unit::generate_address(
        chain,
        "hyperliquid",
        &asset_lower,
        &cfg.address,
        cfg.testnet,
    )
    .await?;

    let fees = unit::estimate_fees(cfg.testnet).await.ok();

    if json_out {
        let mut out = json!({
            "action": "deposit",
            "exchange": "hyperliquid",
            "asset": asset.to_uppercase(),
            "source_chain": chain,
            "destination": "hyperliquid",
            "hl_address": cfg.address,
            "deposit_address": resp.address,
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
            "  {} {} → Hyperliquid",
            "Deposit".green().bold(),
            asset.to_uppercase().cyan()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!("  {} {}", "Source chain:".dimmed(), chain.yellow());
        println!("  {} {}", "HL address:  ".dimmed(), cfg.address.cyan());
        println!("  {} {}", "Minimum:     ".dimmed(), min);
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
            let key_eta = format!("{}-depositEta", chain);
            let key_fee = format!("{}-depositFee", chain);
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
                    &asset.to_lowercase(),
                );
                println!("  {} {}", "Est. fee:    ".dimmed(), fee_str);
            }
        }
        println!();
        println!(
            "  {} This address is permanent — send any amount, any time.",
            "ℹ".blue()
        );
        println!("  {} Track status: fintool bridge-status", "ℹ".blue());
        println!();
    }

    Ok(())
}

// ── USDC bridge via Across → HL ──────────────────────────────────────

async fn deposit_usdc_hl(
    amount: Option<&str>,
    from: Option<&str>,
    dry_run: bool,
    json_out: bool,
) -> Result<()> {
    let cfg = config::load_hl_config()?;

    let amount = amount.ok_or_else(|| {
        anyhow::anyhow!(
            "USDC deposits require --amount.\n\
             Usage: fintool deposit USDC --amount 100 --from ethereum"

        )
    })?;

    let from = from.ok_or_else(|| {
        anyhow::anyhow!(
            "USDC deposits require --from.\n\
             Usage: fintool deposit USDC --amount 100 --from ethereum\n\
             Supported: ethereum, base"
        )
    })?;

    let source: SourceChain = from.parse()?;

    // HL Bridge2 requires minimum 5 USDC — below this is lost forever
    let amount_f: f64 = amount.parse().unwrap_or(0.0);
    if amount_f < 5.0 {
        bail!(
            "HL Bridge2 requires a minimum deposit of 5 USDC.\n\
             Amounts below 5 USDC are not credited and lost forever.\n\
             You requested {} USDC. Please use --amount 5 or higher.",
            amount
        );
    }

    // Step 1: Get Across bridge quote
    eprintln!("Fetching bridge quote from Across...");
    let quote = bridge::get_across_quote(source, amount, &cfg.address).await?;

    let output_amount = quote
        .expected_output_amount
        .as_deref()
        .unwrap_or(&quote.input_amount);
    let fill_time = quote.expected_fill_time.unwrap_or(0);
    let needs_approval = quote.approval_txns.as_ref().is_some_and(|a| !a.is_empty());

    if dry_run {
        // Quote-only mode
        if json_out {
            let mut out = json!({
                "action": "deposit_usdc_quote",
                "exchange": "hyperliquid",
                "source_chain": source.name(),
                "amount_in": bridge::format_usdc(&quote.input_amount),
                "amount_out": bridge::format_usdc(output_amount),
                "expected_fill_time_seconds": fill_time,
                "needs_approval": needs_approval,
                "hl_address": cfg.address,
                "bridge": "across",
            });
            if let Some(ref fees) = quote.fees {
                out["fees"] = fees.clone();
            }
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("{}", "━".repeat(55).dimmed());
            println!(
                "  {} {} USDC  {} → Hyperliquid  {}",
                "Deposit".green().bold(),
                amount,
                source.name().yellow(),
                "(dry run)".dimmed()
            );
            println!("{}", "━".repeat(55).dimmed());
            println!();
            println!(
                "  {} {} → Arbitrum → Hyperliquid",
                "Route:      ".dimmed(),
                source.name()
            );
            println!("  {} Across Protocol", "Bridge:     ".dimmed());
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
            println!("  {} ~{}s", "Fill time:  ".dimmed(), fill_time);
            println!("  {} {}", "HL address: ".dimmed(), cfg.address.cyan());
            println!();
            println!("  {} Remove --dry-run to execute.", "ℹ".blue());
            println!();
        }
        return Ok(());
    }

    // Execute mode
    eprintln!(
        "Executing USDC bridge: {} → Arbitrum → Hyperliquid...",
        source.name()
    );

    let provider_url = source.rpc_url();
    let arb_provider_url = bridge::RPC_ARBITRUM;

    // Build ethers provider + signer for source chain
    let source_provider =
        ethers::providers::Provider::<ethers::providers::Http>::try_from(provider_url)
            .context("Failed to connect to source chain RPC")?;
    let source_wallet = cfg
        .private_key
        .parse::<ethers::signers::LocalWallet>()
        .context("Invalid private key")?
        .with_chain_id(source.chain_id());
    let source_client = std::sync::Arc::new(ethers::middleware::SignerMiddleware::new(
        source_provider,
        source_wallet,
    ));

    // Pre-flight: check ETH for gas on source chain and Arbitrum
    eprintln!("Checking gas balances...");

    let source_eth = bridge::get_eth_balance(source.rpc_url(), &cfg.address).await?;
    let source_eth_f = ethers::utils::format_ether(source_eth);
    eprintln!("  ETH on {}: {}", source.name(), source_eth_f);

    let arb_eth = bridge::get_eth_balance(bridge::RPC_ARBITRUM, &cfg.address).await?;
    let arb_eth_f = ethers::utils::format_ether(arb_eth);
    eprintln!("  ETH on Arbitrum: {}", arb_eth_f);

    // Need ~0.0001 ETH on Arbitrum for the final ERC-20 transfer to HL Bridge2
    let min_arb = ethers::utils::parse_ether("0.0001").unwrap_or_default();
    let need_arb_bridge = arb_eth < min_arb;

    // Source chain needs: gas for USDC approval + bridge (~0.0005 ETH)
    //   + if bridging ETH to Arb: 0.001 ETH value + gas for ETH bridge (~0.0003 ETH)
    let min_source = if need_arb_bridge {
        // 0.0005 gas for USDC txns + 0.001 ETH to bridge + 0.0003 gas for ETH bridge
        ethers::utils::parse_ether("0.002").unwrap_or_default()
    } else {
        ethers::utils::parse_ether("0.0005").unwrap_or_default()
    };
    let min_source_f = ethers::utils::format_ether(min_source);

    if source_eth < min_source {
        if need_arb_bridge {
            bail!(
                "Insufficient ETH on {} for gas. Have {} ETH, need at least {} ETH.\n\
                 This includes ~0.001 ETH to bridge to Arbitrum for the HL deposit step.\n\
                 Send ETH to {} on {} to cover gas fees.",
                source.name(),
                source_eth_f,
                min_source_f,
                cfg.address,
                source.name()
            );
        } else {
            bail!(
                "Insufficient ETH on {} for gas. Have {} ETH, need at least {} ETH.\n\
                 Send ETH to {} on {} to cover gas fees.",
                source.name(),
                source_eth_f,
                min_source_f,
                cfg.address,
                source.name()
            );
        }
    }

    if need_arb_bridge {
        eprintln!(
            "  Insufficient ETH on Arbitrum (have {}, need ~0.0001).",
            arb_eth_f
        );
        eprintln!(
            "  Bridging 0.001 ETH from {} to Arbitrum via Across...",
            source.name()
        );

        // Bridge 0.001 ETH — above Across minimum (~$1), enough for many Arb txns
        let eth_amount_wei = "1000000000000000"; // 0.001 ETH
        let eth_amount_u256 = ethers::types::U256::from_dec_str(eth_amount_wei).unwrap_or_default();
        let eth_quote = bridge::get_eth_bridge_quote(source, eth_amount_wei, &cfg.address).await?;

        // Step 0: Wrap native ETH → WETH (Across requires WETH, not native ETH)
        eprintln!("    Wrapping 0.001 ETH → WETH...");
        let weth_addr: ethers::types::Address = source
            .weth_address()
            .parse()
            .context("Invalid WETH address")?;
        // WETH deposit() selector = 0xd0e30db0
        let wrap_tx = ethers::types::TransactionRequest::new()
            .to(weth_addr)
            .data(hex::decode("d0e30db0")?)
            .value(eth_amount_u256)
            .chain_id(source.chain_id());

        let pending = source_client
            .send_transaction(wrap_tx, None)
            .await
            .context("Failed to wrap ETH → WETH")?;

        let wrap_receipt = pending
            .await
            .context("WETH wrap tx failed")?
            .ok_or_else(|| anyhow::anyhow!("WETH wrap tx dropped"))?;

        eprintln!(
            "    ✅ WETH wrap confirmed: {:?}",
            wrap_receipt.transaction_hash
        );

        // Execute ETH bridge approval txns (if any)
        if let Some(ref eth_approval_txns) = eth_quote.approval_txns {
            for (i, atx) in eth_approval_txns.iter().enumerate() {
                eprintln!(
                    "    ETH bridge approval tx {}/{}...",
                    i + 1,
                    eth_approval_txns.len()
                );
                let tx = ethers::types::TransactionRequest::new()
                    .to(atx
                        .to
                        .parse::<ethers::types::Address>()
                        .context("Invalid ETH bridge approval address")?)
                    .data(
                        hex::decode(atx.data.strip_prefix("0x").unwrap_or(&atx.data))
                            .context("Invalid ETH bridge approval data")?,
                    )
                    .chain_id(source.chain_id());

                let pending = source_client
                    .send_transaction(tx, None)
                    .await
                    .context("Failed to send ETH bridge approval tx")?;

                let receipt = pending
                    .await
                    .context("ETH bridge approval tx failed")?
                    .ok_or_else(|| anyhow::anyhow!("ETH bridge approval tx dropped"))?;

                eprintln!("    ✅ Confirmed: {:?}", receipt.transaction_hash);
            }
        }

        // Execute ETH bridge tx (sends native ETH via Across)
        let eth_bridge_value = eth_quote
            .swap_tx
            .value
            .as_ref()
            .and_then(|v| ethers::types::U256::from_dec_str(v).ok())
            .unwrap_or_default();

        let eth_bridge_tx = ethers::types::TransactionRequest::new()
            .to(eth_quote
                .swap_tx
                .to
                .parse::<ethers::types::Address>()
                .context("Invalid ETH bridge address")?)
            .data(
                hex::decode(
                    eth_quote
                        .swap_tx
                        .data
                        .strip_prefix("0x")
                        .unwrap_or(&eth_quote.swap_tx.data),
                )
                .context("Invalid ETH bridge data")?,
            )
            .value(eth_bridge_value)
            .chain_id(source.chain_id());

        let pending = source_client
            .send_transaction(eth_bridge_tx, None)
            .await
            .context("Failed to send ETH bridge tx")?;

        let eth_receipt = pending
            .await
            .context("ETH bridge tx failed")?
            .ok_or_else(|| anyhow::anyhow!("ETH bridge tx dropped"))?;

        eprintln!(
            "  ✅ ETH bridge tx confirmed: {:?}",
            eth_receipt.transaction_hash
        );

        // Wait for Across relayer to deliver native ETH on Arbitrum
        let eth_fill_time = eth_quote.expected_fill_time.unwrap_or(0).max(10);
        eprintln!(
            "  Waiting for ETH to arrive on Arbitrum (~{}s)...",
            eth_fill_time
        );
        tokio::time::sleep(tokio::time::Duration::from_secs(eth_fill_time)).await;
        eprintln!("  ✅ ETH bridged to Arbitrum for gas");
    }

    // Step 2: Approval txns (if needed)
    if let Some(ref approval_txns) = quote.approval_txns {
        for (i, atx) in approval_txns.iter().enumerate() {
            eprintln!("  Sending approval tx {}/{}...", i + 1, approval_txns.len());
            let tx = ethers::types::TransactionRequest::new()
                .to(atx
                    .to
                    .parse::<ethers::types::Address>()
                    .context("Invalid approval address")?)
                .data(
                    hex::decode(atx.data.strip_prefix("0x").unwrap_or(&atx.data))
                        .context("Invalid approval data")?,
                )
                .chain_id(source.chain_id());

            let pending = source_client
                .send_transaction(tx, None)
                .await
                .context("Failed to send approval tx")?;

            let receipt = pending
                .await
                .context("Approval tx failed")?
                .ok_or_else(|| anyhow::anyhow!("Approval tx dropped"))?;

            eprintln!("  ✅ Approval tx confirmed: {:?}", receipt.transaction_hash);
        }
    }

    // Step 3: Bridge tx (Across SpokePool on source chain)
    eprintln!("  Sending bridge tx on {}...", source.name());
    let bridge_value = quote
        .swap_tx
        .value
        .as_ref()
        .and_then(|v| ethers::types::U256::from_dec_str(v).ok())
        .unwrap_or_default();

    let bridge_tx = ethers::types::TransactionRequest::new()
        .to(quote
            .swap_tx
            .to
            .parse::<ethers::types::Address>()
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
        .chain_id(source.chain_id());

    let pending = source_client
        .send_transaction(bridge_tx, None)
        .await
        .context("Failed to send bridge tx")?;

    let bridge_receipt = pending
        .await
        .context("Bridge tx failed")?
        .ok_or_else(|| anyhow::anyhow!("Bridge tx dropped"))?;

    let bridge_tx_hash = format!("{:?}", bridge_receipt.transaction_hash);
    eprintln!("  ✅ Bridge tx confirmed: {}", bridge_tx_hash);

    // Step 4: Wait for Across relayer to fill on Arbitrum
    eprintln!("  Waiting for Across relayer (~{}s)...", fill_time.max(5));
    tokio::time::sleep(tokio::time::Duration::from_secs(fill_time.max(10))).await;

    // Step 5: Send USDC from Arbitrum address to HL Bridge2
    eprintln!("  Sending USDC to HL Bridge2 on Arbitrum...");

    let arb_provider =
        ethers::providers::Provider::<ethers::providers::Http>::try_from(arb_provider_url)
            .context("Failed to connect to Arbitrum RPC")?;
    let arb_wallet = cfg
        .private_key
        .parse::<ethers::signers::LocalWallet>()
        .context("Invalid private key")?
        .with_chain_id(bridge::ARBITRUM_CHAIN_ID);
    let arb_client = std::sync::Arc::new(ethers::middleware::SignerMiddleware::new(
        arb_provider,
        arb_wallet,
    ));

    // ERC-20 transfer USDC to HL Bridge2
    let transfer_data = bridge::encode_erc20_transfer(bridge::HL_BRIDGE2_MAINNET, output_amount)?;

    // Query current gas price from Arbitrum and add 50% buffer to avoid
    // "max fee per gas less than block base fee" errors
    let arb_gas_price = arb_client
        .get_gas_price()
        .await
        .context("Failed to fetch Arbitrum gas price")?;
    let arb_gas_price_buffered = arb_gas_price * 150 / 100;

    let hl_deposit_tx = ethers::types::TransactionRequest::new()
        .to(bridge::USDC_ARBITRUM
            .parse::<ethers::types::Address>()
            .context("Invalid USDC address")?)
        .data(transfer_data)
        .chain_id(bridge::ARBITRUM_CHAIN_ID)
        .gas_price(arb_gas_price_buffered);

    let pending = arb_client
        .send_transaction(hl_deposit_tx, None)
        .await
        .context("Failed to send HL deposit tx")?;

    let hl_receipt = pending
        .await
        .context("HL deposit tx failed")?
        .ok_or_else(|| anyhow::anyhow!("HL deposit tx dropped"))?;

    let hl_tx_hash = format!("{:?}", hl_receipt.transaction_hash);
    eprintln!("  ✅ HL deposit tx confirmed: {}", hl_tx_hash);

    // Output result
    if json_out {
        let out = json!({
            "action": "deposit_usdc",
            "exchange": "hyperliquid",
            "status": "completed",
            "source_chain": source.name(),
            "amount_in": bridge::format_usdc(&quote.input_amount),
            "amount_deposited": bridge::format_usdc(output_amount),
            "hl_address": cfg.address,
            "bridge_tx": bridge_tx_hash,
            "hl_deposit_tx": hl_tx_hash,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!();
        println!("{}", "━".repeat(55).dimmed());
        println!(
            "  {} {} USDC deposited to Hyperliquid",
            "✅".green(),
            bridge::format_usdc(output_amount).green().bold()
        );
        println!("{}", "━".repeat(55).dimmed());
        println!();
        println!("  {} {}", "Source:     ".dimmed(), source.name());
        println!(
            "  {} {}",
            "Amount in:  ".dimmed(),
            bridge::format_usdc(&quote.input_amount)
        );
        println!(
            "  {} {}",
            "Deposited:  ".dimmed(),
            bridge::format_usdc(output_amount).green()
        );
        println!("  {} {}", "Bridge TX:  ".dimmed(), bridge_tx_hash.cyan());
        println!("  {} {}", "HL TX:      ".dimmed(), hl_tx_hash.cyan());
        println!();
    }

    Ok(())
}

// ── Binance deposit address ──────────────────────────────────────────

async fn deposit_binance(asset: &str, network: Option<&str>, json_out: bool) -> Result<()> {
    let (api_key, api_secret) = config::binance_credentials().ok_or_else(|| {
        anyhow::anyhow!("Binance API keys not configured in ~/.fintool/config.toml")
    })?;

    let client = reqwest::Client::new();

    // Map common chain names to Binance network codes
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

    let resp = binance::get_deposit_address(
        &client,
        &api_key,
        &api_secret,
        asset,
        binance_network.as_deref(),
    )
    .await?;

    let address = resp["address"].as_str().unwrap_or("unknown");
    let tag = resp["tag"].as_str().unwrap_or("");
    let coin = resp["coin"].as_str().unwrap_or(asset);

    if json_out {
        let mut out = json!({
            "action": "deposit",
            "exchange": "binance",
            "asset": coin,
            "deposit_address": address,
        });
        if !tag.is_empty() {
            out["tag"] = json!(tag);
        }
        if let Some(net) = binance_network.as_deref() {
            out["network"] = json!(net);
        }
        if let Some(url) = resp["url"].as_str() {
            out["explorer_url"] = json!(url);
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", "━".repeat(50).dimmed());
        println!("  {} {} → Binance", "Deposit".green().bold(), coin.cyan());
        println!("{}", "━".repeat(50).dimmed());
        println!();
        if let Some(net) = binance_network.as_deref() {
            println!("  {} {}", "Network:    ".dimmed(), net.yellow());
        }
        println!();
        println!("  {} Send {} to:", "→".green().bold(), coin);
        println!();
        println!("    {}", address.green().bold());
        if !tag.is_empty() {
            println!("    {} {}", "Tag/Memo:".dimmed(), tag.yellow().bold());
        }
        println!();
    }

    Ok(())
}

// ── Coinbase deposit address ─────────────────────────────────────────

async fn deposit_coinbase(asset: &str, json_out: bool) -> Result<()> {
    let (api_key, api_secret) = config::coinbase_credentials().ok_or_else(|| {
        anyhow::anyhow!("Coinbase API keys not configured in ~/.fintool/config.toml")
    })?;

    let client = reqwest::Client::new();

    // First, find the account for this asset
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

    // Generate deposit address
    let resp =
        coinbase::create_deposit_address(&client, &api_key, &api_secret, &account_id).await?;

    let address = resp["data"]["address"]
        .as_str()
        .or_else(|| resp["address"].as_str())
        .unwrap_or("unknown");

    let network = resp["data"]["network"].as_str().unwrap_or("");

    if json_out {
        let mut out = json!({
            "action": "deposit",
            "exchange": "coinbase",
            "asset": asset.to_uppercase(),
            "deposit_address": address,
            "account_id": account_id,
        });
        if !network.is_empty() {
            out["network"] = json!(network);
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", "━".repeat(50).dimmed());
        println!(
            "  {} {} → Coinbase",
            "Deposit".green().bold(),
            asset.to_uppercase().cyan()
        );
        println!("{}", "━".repeat(50).dimmed());
        println!();
        if !network.is_empty() {
            println!("  {} {}", "Network:    ".dimmed(), network.yellow());
        }
        println!();
        println!("  {} Send {} to:", "→".green().bold(), asset.to_uppercase());
        println!();
        println!("    {}", address.green().bold());
        println!();
    }

    Ok(())
}
