//! HIP-3 (builder-deployed perpetuals) order signing and execution
//!
//! The Hyperliquid Rust SDK doesn't support HIP-3 dexes, so we implement
//! the signing and order submission directly.
//!
//! Signing flow:
//! 1. Build order action (same wire format as SDK)
//! 2. Msgpack-serialize → append timestamp BE bytes → append 0x00 (no vault)
//! 3. Keccak256 → connection_id
//! 4. EIP-712 sign Agent { source: "a"|"b", connection_id }
//! 5. POST to /exchange

use anyhow::{Context, Result};
use ethers::{
    abi::{encode, ParamType, Tokenizable},
    signers::LocalWallet,
    types::{
        transaction::eip712::{
            encode_eip712_type, make_type_hash, EIP712Domain, Eip712, Eip712Error,
        },
        H256, U256,
    },
    utils::keccak256,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config;

// ── EIP-712 Agent struct (matches SDK) ───────────────────────────────

#[derive(Debug)]
struct Agent {
    source: String,
    connection_id: H256,
}

impl Eip712 for Agent {
    type Error = Eip712Error;

    fn domain(&self) -> std::result::Result<EIP712Domain, Self::Error> {
        Ok(EIP712Domain {
            name: Some("Exchange".to_string()),
            version: Some("1".to_string()),
            chain_id: Some(U256::from(1337)),
            verifying_contract: Some(
                "0x0000000000000000000000000000000000000000"
                    .parse()
                    .unwrap(),
            ),
            salt: None,
        })
    }

    fn type_hash() -> std::result::Result<[u8; 32], Self::Error> {
        Ok(make_type_hash(
            "Agent".to_string(),
            &[
                ("source".to_string(), ParamType::String),
                ("connectionId".to_string(), ParamType::FixedBytes(32)),
            ],
        ))
    }

    fn struct_hash(&self) -> std::result::Result<[u8; 32], Self::Error> {
        let items = vec![
            ethers::abi::Token::Uint(Self::type_hash()?.into()),
            encode_eip712_type(self.source.clone().into_token()),
            encode_eip712_type(ethers::abi::Token::FixedBytes(
                self.connection_id.as_bytes().to_vec(),
            )),
        ];
        Ok(keccak256(encode(&items)))
    }
}

// ── Msgpack order format (matches SDK wire format) ───────────────────

#[derive(Serialize, Deserialize, Debug)]
struct OrderWire {
    a: u32,
    b: bool,
    p: String,
    s: String,
    r: bool,
    t: OrderType,
}

#[derive(Serialize, Deserialize, Debug)]
struct OrderType {
    limit: LimitType,
}

#[derive(Serialize, Deserialize, Debug)]
struct LimitType {
    tif: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OrderAction {
    #[serde(rename = "type")]
    action_type: String,
    orders: Vec<OrderWire>,
    grouping: String,
}

// ── Resolve HIP-3 asset index ────────────────────────────────────────

/// Get all HIP-3 dexes and find the offset for a specific dex
async fn get_dex_offset(http: &reqwest::Client, url: &str, dex: &str) -> Result<u32> {
    let dexes: Vec<Value> = http
        .post(url)
        .json(&json!({"type": "perpDexs"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse perpDexs")?;

    let mut dex_idx: u32 = 0;
    for entry in &dexes {
        if entry.is_null() {
            continue; // skip the null (main dex) entry
        }
        if entry.get("name").and_then(|n| n.as_str()) == Some(dex) {
            return Ok(110000 + dex_idx * 10000);
        }
        dex_idx += 1;
    }

    anyhow::bail!("HIP-3 dex '{}' not found", dex)
}

/// Get the asset index within a HIP-3 dex and its szDecimals
async fn get_asset_in_dex(
    http: &reqwest::Client,
    url: &str,
    dex: &str,
    asset_name: &str,
) -> Result<(u32, u32)> {
    let meta: Value = http
        .post(url)
        .json(&json!({"type": "meta", "dex": dex}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse HIP-3 meta")?;

    if let Some(universe) = meta.get("universe").and_then(|u| u.as_array()) {
        for (i, asset) in universe.iter().enumerate() {
            if asset.get("name").and_then(|n| n.as_str()) == Some(asset_name) {
                let sz_decimals = asset
                    .get("szDecimals")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(4) as u32;
                return Ok((i as u32, sz_decimals));
            }
        }
    }

    anyhow::bail!("Asset '{}' not found in HIP-3 dex '{}'", asset_name, dex)
}

/// Resolve the global asset index for a HIP-3 asset
pub async fn resolve_hip3_asset(dex: &str, asset_name: &str) -> Result<(u32, u32)> {
    let http = reqwest::Client::new();
    let url = config::info_url();

    let offset = get_dex_offset(&http, &url, dex).await?;
    let (local_idx, sz_decimals) = get_asset_in_dex(&http, &url, dex, asset_name).await?;

    Ok((offset + local_idx, sz_decimals))
}

// ── Format price/size like the SDK ───────────────────────────────────

fn float_to_wire(val: f64, sz_decimals: u32) -> String {
    // Match SDK: 5 significant figures, then round to appropriate decimals
    let formatted = format!("{:.prec$}", val, prec = 5);
    let parsed: f64 = formatted.parse().unwrap_or(val);
    // For prices: 6 - sz_decimals decimal places (perps)
    // For sizes: sz_decimals decimal places
    format!("{:.prec$}", parsed, prec = sz_decimals as usize)
}

fn price_to_wire(price: f64, sz_decimals: u32) -> String {
    // Prices use (6 - szDecimals) significant decimals for perps
    let max_decimals = if sz_decimals < 6 { 6 - sz_decimals } else { 0 };
    let s = format!("{:.5}", price); // 5 sig figures first
    let parsed: f64 = s.parse().unwrap_or(price);
    format!("{:.prec$}", parsed, prec = max_decimals as usize)
}

// ── Sign and submit order ────────────────────────────────────────────

/// Place a HIP-3 perp limit order
pub async fn place_order(
    dex: &str,
    asset_name: &str,
    is_buy: bool,
    price: f64,
    size: f64,
) -> Result<Value> {
    let cfg = config::load_hl_config()?;
    let wallet: LocalWallet = cfg.private_key.parse().context("Invalid private key")?;
    let is_mainnet = !cfg.testnet;

    // Resolve asset index
    let (asset_index, sz_decimals) = resolve_hip3_asset(dex, asset_name).await?;

    // Build order action for msgpack
    let action = OrderAction {
        action_type: "order".to_string(),
        orders: vec![OrderWire {
            a: asset_index,
            b: is_buy,
            p: price_to_wire(price, sz_decimals),
            s: float_to_wire(size, sz_decimals),
            r: false,
            t: OrderType {
                limit: LimitType {
                    tif: "Gtc".to_string(),
                },
            },
        }],
        grouping: "na".to_string(),
    };

    // Compute connection_id: keccak256(msgpack || timestamp_be || 0x00)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    let mut bytes =
        rmp_serde::to_vec_named(&action).context("Failed to msgpack serialize order")?;
    bytes.extend(timestamp.to_be_bytes());
    bytes.push(0u8); // no vault address
    let connection_id = H256(keccak256(bytes));

    // EIP-712 sign
    let source = if is_mainnet { "a" } else { "b" }.to_string();
    let agent = Agent {
        source,
        connection_id,
    };

    let encoded = agent
        .encode_eip712()
        .map_err(|e| anyhow::anyhow!("EIP-712 encode failed: {:?}", e))?;

    let signature = wallet
        .sign_hash(H256::from(encoded))
        .map_err(|e| anyhow::anyhow!("Signing failed: {:?}", e))?;

    // Build JSON action for the API (same structure, different format)
    let json_action = json!({
        "type": "order",
        "orders": [{
            "a": asset_index,
            "b": is_buy,
            "p": price_to_wire(price, sz_decimals),
            "s": float_to_wire(size, sz_decimals),
            "r": false,
            "t": {
                "limit": {
                    "tif": "Gtc"
                }
            }
        }],
        "grouping": "na"
    });

    let api_url = if is_mainnet {
        "https://api.hyperliquid.xyz/exchange"
    } else {
        "https://api.hyperliquid-testnet.xyz/exchange"
    };

    let sig_json = json!({
        "r": format!("0x{:064x}", signature.r),
        "s": format!("0x{:064x}", signature.s),
        "v": signature.v,
    });

    let payload = json!({
        "action": json_action,
        "nonce": timestamp,
        "signature": sig_json,
        "vaultAddress": null,
    });

    let http = reqwest::Client::builder()
        .user_agent("fintool/0.1")
        .build()?;

    let resp = http
        .post(api_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to send HIP-3 order")?;

    let status = resp.status();
    let body: Value = resp.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        anyhow::bail!("HIP-3 order failed ({}): {:?}", status, body);
    }

    Ok(body)
}
