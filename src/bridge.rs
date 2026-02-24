//! Cross-chain USDC bridge via Across Protocol + HL Bridge2 deposit
//!
//! Flow:
//! 1. Query Across API for bridge quote + calldata
//! 2. Approve USDC spend if needed (ERC-20 approve tx on source chain)
//! 3. Execute bridge tx on source chain (Across SpokePool)
//! 4. USDC arrives on Arbitrum (~2-10 seconds via Across relayers)
//! 5. Send USDC from user's Arbitrum address to HL Bridge2 contract
//!
//! Alternatively, step 4+5 can be done as: Across deposits to user on Arb,
//! then user sends to HL bridge. Both automated with the same private key.

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Chain constants ──────────────────────────────────────────────────

pub const ETHEREUM_CHAIN_ID: u64 = 1;
pub const BASE_CHAIN_ID: u64 = 8453;
pub const ARBITRUM_CHAIN_ID: u64 = 42161;

// USDC contract addresses
pub const USDC_ETHEREUM: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const USDC_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
pub const USDC_ARBITRUM: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";

// HL Bridge2 on Arbitrum
pub const HL_BRIDGE2_MAINNET: &str = "0x2df1c51e09aecf9cacb7bc98cb1742757f163df7";
pub const HL_BRIDGE2_TESTNET: &str = "0x08cfc1B6b2dCF36A1480b99353A354AA8AC56f89";

// Across API
const ACROSS_API: &str = "https://app.across.to/api";

// Default public RPC endpoints
pub const RPC_ETHEREUM: &str = "https://eth.llamarpc.com";
pub const RPC_BASE: &str = "https://mainnet.base.org";
pub const RPC_ARBITRUM: &str = "https://arb1.arbitrum.io/rpc";

// ── Types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum SourceChain {
    Ethereum,
    Base,
}

impl SourceChain {
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Ethereum => ETHEREUM_CHAIN_ID,
            Self::Base => BASE_CHAIN_ID,
        }
    }

    pub fn usdc_address(&self) -> &'static str {
        match self {
            Self::Ethereum => USDC_ETHEREUM,
            Self::Base => USDC_BASE,
        }
    }

    pub fn rpc_url(&self) -> &'static str {
        match self {
            Self::Ethereum => RPC_ETHEREUM,
            Self::Base => RPC_BASE,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Base => "base",
        }
    }
}

impl std::str::FromStr for SourceChain {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ethereum" | "eth" | "mainnet" => Ok(Self::Ethereum),
            "base" => Ok(Self::Base),
            _ => bail!("Unsupported source chain '{}'. Use: ethereum, base", s),
        }
    }
}

// ── Across API ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AcrossSwapResponse {
    #[serde(rename = "crossSwapType")]
    pub cross_swap_type: Option<String>,
    #[serde(rename = "approvalTxns")]
    pub approval_txns: Option<Vec<AcrossTx>>,
    #[serde(rename = "swapTx")]
    pub swap_tx: AcrossTx,
    #[serde(rename = "inputAmount")]
    pub input_amount: String,
    #[serde(rename = "expectedOutputAmount")]
    pub expected_output_amount: Option<String>,
    #[serde(rename = "expectedFillTime")]
    pub expected_fill_time: Option<u64>,
    pub fees: Option<Value>,
    pub checks: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AcrossTx {
    #[serde(rename = "chainId")]
    pub chain_id: Option<u64>,
    pub to: String,
    pub data: String,
    pub value: Option<String>,
    #[serde(rename = "maxFeePerGas")]
    pub max_fee_per_gas: Option<String>,
    #[serde(rename = "maxPriorityFeePerGas")]
    pub max_priority_fee_per_gas: Option<String>,
}

fn client() -> Result<Client> {
    Client::builder()
        .user_agent("fintool/0.1")
        .build()
        .context("Failed to build HTTP client")
}

/// Get bridge quote and executable calldata from Across API
/// Bridges USDC from source chain → USDC on Arbitrum
pub async fn get_across_quote(
    source: SourceChain,
    amount_usdc: &str,
    depositor: &str,
) -> Result<AcrossSwapResponse> {
    // Convert USDC amount to smallest unit (6 decimals)
    let amount_raw = parse_usdc_amount(amount_usdc)?;

    let url = format!("{}/swap/approval", ACROSS_API);
    let resp = client()?
        .get(&url)
        .query(&[
            ("tradeType", "exactInput"),
            ("amount", &amount_raw),
            ("inputToken", source.usdc_address()),
            ("originChainId", &source.chain_id().to_string()),
            ("outputToken", USDC_ARBITRUM),
            ("destinationChainId", &ARBITRUM_CHAIN_ID.to_string()),
            ("depositor", depositor),
        ])
        .send()
        .await
        .context("Failed to call Across API")?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Across API error ({}): {}", status, body);
    }

    serde_json::from_str(&body).context("Failed to parse Across response")
}

/// Parse a human-readable USDC amount (e.g. "100" or "100.50") to raw units (6 decimals)
fn parse_usdc_amount(amount: &str) -> Result<String> {
    let parts: Vec<&str> = amount.split('.').collect();
    match parts.len() {
        1 => {
            let whole: u64 = parts[0].parse().context("Invalid USDC amount")?;
            Ok((whole * 1_000_000).to_string())
        }
        2 => {
            let whole: u64 = parts[0].parse().context("Invalid USDC amount")?;
            let mut frac = parts[1].to_string();
            // Pad or truncate to 6 decimal places
            while frac.len() < 6 {
                frac.push('0');
            }
            frac.truncate(6);
            let frac_val: u64 = frac.parse().context("Invalid USDC decimal")?;
            Ok((whole * 1_000_000 + frac_val).to_string())
        }
        _ => bail!("Invalid USDC amount: {}", amount),
    }
}

/// Format raw USDC amount (6 decimals) to human-readable
pub fn format_usdc(raw: &str) -> String {
    let val: u64 = raw.parse().unwrap_or(0);
    let whole = val / 1_000_000;
    let frac = val % 1_000_000;
    if frac == 0 {
        format!("{} USDC", whole)
    } else {
        let decimal = format!("{:06}", frac).trim_end_matches('0').to_string();
        format!("{}.{} USDC", whole, decimal)
    }
}

// ── ERC-20 ABI helpers ───────────────────────────────────────────────

/// Encode ERC-20 transfer(address,uint256) calldata
pub fn encode_erc20_transfer(to: &str, amount_raw: &str) -> Result<Vec<u8>> {
    use ethers::abi::{encode, Token};
    let to_addr: ethers::types::Address = to.parse().context("Invalid address")?;
    let amount: ethers::types::U256 =
        ethers::types::U256::from_dec_str(amount_raw).context("Invalid amount")?;

    // transfer(address,uint256) selector = 0xa9059cbb
    let selector = hex::decode("a9059cbb")?;
    let encoded = encode(&[Token::Address(to_addr), Token::Uint(amount)]);

    let mut calldata = selector;
    calldata.extend_from_slice(&encoded);
    Ok(calldata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_usdc_amount() {
        assert_eq!(parse_usdc_amount("100").unwrap(), "100000000");
        assert_eq!(parse_usdc_amount("100.50").unwrap(), "100500000");
        assert_eq!(parse_usdc_amount("0.01").unwrap(), "10000");
        assert_eq!(parse_usdc_amount("1000").unwrap(), "1000000000");
    }

    #[test]
    fn test_format_usdc() {
        assert_eq!(format_usdc("100000000"), "100 USDC");
        assert_eq!(format_usdc("100500000"), "100.5 USDC");
        assert_eq!(format_usdc("10000"), "0.01 USDC");
    }
}
