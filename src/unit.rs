//! HyperUnit (Unit) bridge — deposit/withdraw ETH, BTC, SOL to/from Hyperliquid
//! API docs: https://docs.hyperunit.xyz/developers/api

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAINNET_API: &str = "https://api.hyperunit.xyz";
const TESTNET_API: &str = "https://api.hyperunit-testnet.xyz";

/// Supported assets for Unit bridge
pub fn supported_assets() -> &'static [&'static str] {
    &["eth", "btc", "sol"]
}

/// Check if an asset is supported by Unit
pub fn is_supported(asset: &str) -> bool {
    supported_assets().contains(&asset.to_lowercase().as_str())
}

fn api_base(testnet: bool) -> &'static str {
    if testnet {
        TESTNET_API
    } else {
        MAINNET_API
    }
}

fn client() -> Result<Client> {
    Client::builder()
        .user_agent("fintool/0.1")
        .build()
        .context("Failed to build HTTP client")
}

/// Minimum deposit/withdrawal amounts
pub fn minimum_amount(asset: &str) -> Option<&'static str> {
    match asset.to_lowercase().as_str() {
        "eth" => Some("0.007 ETH"),
        "btc" => Some("0.0003 BTC"),
        "sol" => Some("0.12 SOL"),
        _ => None,
    }
}

// ── Generate deposit/withdrawal address ──────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct GenAddressResponse {
    pub address: String,
    pub signatures: Value,
    pub status: Option<String>,
    pub error: Option<String>,
}

/// Generate a Unit deposit address.
/// For deposits: src_chain = "ethereum"/"bitcoin"/"solana", dst_chain = "hyperliquid"
/// For withdrawals: src_chain = "hyperliquid", dst_chain = "ethereum"/"bitcoin"/"solana"
pub async fn generate_address(
    src_chain: &str,
    dst_chain: &str,
    asset: &str,
    dst_addr: &str,
    testnet: bool,
) -> Result<GenAddressResponse> {
    let url = format!(
        "{}/gen/{}/{}/{}/{}",
        api_base(testnet),
        src_chain,
        dst_chain,
        asset.to_lowercase(),
        dst_addr
    );
    let resp = client()?
        .get(&url)
        .send()
        .await
        .context("Failed to call Unit API")?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Unit API error ({}): {}", status, body);
    }

    let parsed: GenAddressResponse =
        serde_json::from_str(&body).context("Failed to parse Unit response")?;

    if let Some(ref err) = parsed.error {
        bail!("Unit API error: {}", err);
    }

    Ok(parsed)
}

// ── Estimate fees ────────────────────────────────────────────────────

pub async fn estimate_fees(testnet: bool) -> Result<Value> {
    let url = format!("{}/v2/estimate-fees", api_base(testnet));
    let resp = client()?
        .get(&url)
        .send()
        .await
        .context("Failed to call Unit fee API")?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Unit fee API error ({}): {}", status, body);
    }

    serde_json::from_str(&body).context("Failed to parse fee response")
}

// ── Operations (track deposit/withdrawal status) ─────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationsResponse {
    pub addresses: Vec<Value>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Operation {
    #[serde(rename = "opCreatedAt")]
    pub op_created_at: String,
    #[serde(rename = "operationId")]
    pub operation_id: String,
    #[serde(rename = "protocolAddress")]
    pub protocol_address: String,
    #[serde(rename = "sourceAddress")]
    pub source_address: String,
    #[serde(rename = "destinationAddress")]
    pub destination_address: String,
    #[serde(rename = "sourceChain")]
    pub source_chain: String,
    #[serde(rename = "destinationChain")]
    pub destination_chain: String,
    #[serde(rename = "sourceAmount")]
    pub source_amount: String,
    #[serde(rename = "destinationFeeAmount")]
    pub destination_fee_amount: String,
    #[serde(rename = "sweepFeeAmount")]
    pub sweep_fee_amount: String,
    #[serde(rename = "sourceTxHash")]
    pub source_tx_hash: String,
    #[serde(rename = "destinationTxHash")]
    pub destination_tx_hash: Option<String>,
    pub asset: String,
    pub state: String,
    #[serde(rename = "sourceTxConfirmations")]
    pub source_tx_confirmations: Option<u64>,
}

/// Get all operations for an address
pub async fn get_operations(address: &str, testnet: bool) -> Result<OperationsResponse> {
    let url = format!("{}/operations/{}", api_base(testnet), address);
    let resp = client()?
        .get(&url)
        .send()
        .await
        .context("Failed to call Unit operations API")?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Unit operations API error ({}): {}", status, body);
    }

    serde_json::from_str(&body).context("Failed to parse operations response")
}

/// Map asset to its native chain
pub fn native_chain(asset: &str) -> Option<&'static str> {
    match asset.to_lowercase().as_str() {
        "eth" => Some("ethereum"),
        "btc" => Some("bitcoin"),
        "sol" => Some("solana"),
        _ => None,
    }
}

/// Format amount from smallest units to human-readable
pub fn format_amount(raw: &str, asset: &str) -> String {
    let val: f64 = raw.parse().unwrap_or(0.0);
    match asset.to_lowercase().as_str() {
        "eth" => format!("{:.6} ETH", val / 1e18),
        "btc" => format!("{:.8} BTC", val / 1e8),
        "sol" => format!("{:.4} SOL", val / 1e9),
        _ => raw.to_string(),
    }
}
