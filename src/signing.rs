use anyhow::{Context, Result};
use ethers::signers::{LocalWallet, Signer};
use hyperliquid_rust_sdk::{
    BaseUrl, ClientCancelRequest, ClientLimit, ClientOrder, ClientOrderRequest, ExchangeClient,
    ExchangeResponseStatus,
};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::config;

/// Create a LocalWallet from config
pub fn get_wallet() -> Result<LocalWallet> {
    let cfg = config::load_hl_config()?;
    let wallet: LocalWallet = cfg
        .private_key
        .parse()
        .context("Failed to parse private key")?;
    Ok(wallet)
}

/// Get the wallet address as a lowercase hex string (matches what the API recovers from signatures)
pub fn get_wallet_address() -> Result<String> {
    let wallet = get_wallet()?;
    Ok(format!("{:?}", wallet.address()).to_lowercase())
}

/// Get the base URL from config
pub fn get_base_url() -> Result<BaseUrl> {
    let cfg = config::load_config_file()?;
    if cfg.network.testnet {
        Ok(BaseUrl::Testnet)
    } else {
        Ok(BaseUrl::Mainnet)
    }
}

/// Create an ExchangeClient ready for trading
pub async fn get_exchange_client() -> Result<ExchangeClient> {
    let wallet = get_wallet()?;
    let base_url = get_base_url()?;
    let client = ExchangeClient::new(None, wallet, Some(base_url), None, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create exchange client: {:?}", e))?;
    Ok(client)
}

/// Resolve a symbol to its spot asset name (e.g. "TSLA" -> "TSLA/USDC")
pub async fn resolve_spot_name(symbol: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let url = config::info_url();

    let spot_meta: Value = client
        .post(&url)
        .json(&json!({"type": "spotMeta"}))
        .send()
        .await?
        .json()
        .await?;

    let mut idx_to_name = HashMap::new();
    if let Some(tokens) = spot_meta.get("tokens").and_then(|t| t.as_array()) {
        for token in tokens {
            if let (Some(idx), Some(name)) = (
                token.get("index").and_then(|i| i.as_u64()),
                token.get("name").and_then(|n| n.as_str()),
            ) {
                idx_to_name.insert(idx, name.to_string());
            }
        }
    }

    if let Some(universe) = spot_meta.get("universe").and_then(|u| u.as_array()) {
        for pair in universe {
            if let Some(tokens) = pair.get("tokens").and_then(|t| t.as_array()) {
                if tokens.len() == 2 {
                    let t1 = tokens[0].as_u64().unwrap_or(0);
                    let t2 = tokens[1].as_u64().unwrap_or(0);
                    let name1 = idx_to_name.get(&t1).map(|s| s.as_str()).unwrap_or("");
                    let name2 = idx_to_name.get(&t2).map(|s| s.as_str()).unwrap_or("");
                    if name1.eq_ignore_ascii_case(symbol) {
                        return Ok(format!("{}/{}", name1, name2));
                    }
                }
            }
        }
    }

    anyhow::bail!("Symbol {} not found in Hyperliquid spot markets", symbol)
}

/// Fetch szDecimals for a spot token
async fn get_spot_sz_decimals(symbol: &str) -> Result<u32> {
    let client = reqwest::Client::new();
    let url = config::info_url();

    let spot_meta: Value = client
        .post(&url)
        .json(&json!({"type": "spotMeta"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse spotMeta")?;

    if let Some(tokens) = spot_meta.get("tokens").and_then(|t| t.as_array()) {
        for token in tokens {
            if token.get("name").and_then(|n| n.as_str()) == Some(symbol) {
                return Ok(token
                    .get("szDecimals")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(4) as u32);
            }
        }
    }

    Ok(4) // default
}

/// Place a spot limit order
pub async fn place_spot_order(
    symbol: &str,
    is_buy: bool,
    price: f64,
    size: f64,
) -> Result<ExchangeResponseStatus> {
    let asset_name = resolve_spot_name(symbol).await?;
    let sz_decimals = get_spot_sz_decimals(symbol).await?;
    let size = truncate_size(size, sz_decimals);
    let price = round_price(price);

    if size == 0.0 {
        anyhow::bail!(
            "Order size too small: rounds to 0 with {} decimal places. Increase the USD amount.",
            sz_decimals
        );
    }

    let client = get_exchange_client().await?;

    let order = ClientOrderRequest {
        asset: asset_name,
        is_buy,
        reduce_only: false,
        limit_px: price,
        sz: size,
        cloid: None,
        order_type: ClientOrder::Limit(ClientLimit {
            tif: "Gtc".to_string(),
        }),
    };

    let result = client
        .order(order, None)
        .await
        .map_err(|e| anyhow::anyhow!("Spot order failed: {:?}", e))?;
    Ok(result)
}

/// Fetch szDecimals for a main perp asset
async fn get_perp_sz_decimals(symbol: &str) -> Result<u32> {
    let client = reqwest::Client::new();
    let url = config::info_url();

    let meta: Value = client
        .post(&url)
        .json(&json!({"type": "meta"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse perp meta")?;

    if let Some(universe) = meta.get("universe").and_then(|u| u.as_array()) {
        for asset in universe {
            if asset.get("name").and_then(|n| n.as_str()) == Some(symbol) {
                return Ok(asset
                    .get("szDecimals")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(4) as u32);
            }
        }
    }

    Ok(4) // default to 4 if not found
}

/// Truncate size to szDecimals (round down to avoid over-spending)
/// Uses small epsilon to handle float precision (e.g. "0.0048" → 0.004799999... → 0.0048)
fn truncate_size(size: f64, sz_decimals: u32) -> f64 {
    let factor = 10f64.powi(sz_decimals as i32);
    (size * factor + 1e-9).floor() / factor
}

/// Round price to 5 significant figures (Hyperliquid tick size rule)
fn round_price(price: f64) -> f64 {
    if price == 0.0 {
        return 0.0;
    }
    let magnitude = price.abs().log10().floor() as i32;
    let decimals = (4 - magnitude).max(0);
    let factor = 10f64.powi(decimals);
    (price * factor).round() / factor
}

/// Place a perp limit order
pub async fn place_perp_order(
    symbol: &str,
    is_buy: bool,
    price: f64,
    size: f64,
    reduce_only: bool,
) -> Result<ExchangeResponseStatus> {
    let sz_decimals = get_perp_sz_decimals(symbol).await?;
    let size = truncate_size(size, sz_decimals);
    let price = round_price(price);

    if size == 0.0 {
        anyhow::bail!(
            "Order size too small: rounds to 0 with {} decimal places. Increase the USD amount.",
            sz_decimals
        );
    }

    let client = get_exchange_client().await?;

    let order = ClientOrderRequest {
        asset: symbol.to_string(),
        is_buy,
        reduce_only,
        limit_px: price,
        sz: size,
        cloid: None,
        order_type: ClientOrder::Limit(ClientLimit {
            tif: "Gtc".to_string(),
        }),
    };

    let result = client
        .order(order, None)
        .await
        .map_err(|e| anyhow::anyhow!("Perp order failed: {:?}", e))?;
    Ok(result)
}

/// Set leverage for a perp asset on Hyperliquid
pub async fn set_leverage(
    symbol: &str,
    leverage: u32,
    is_cross: bool,
) -> Result<ExchangeResponseStatus> {
    let client = get_exchange_client().await?;
    let result = client
        .update_leverage(leverage, symbol, is_cross, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to set leverage: {:?}", e))?;
    Ok(result)
}

/// Transfer USDC between perp and spot accounts
/// to_perp=true: spot → perp, to_perp=false: perp → spot
pub async fn class_transfer(usdc: f64, to_perp: bool) -> Result<()> {
    use ethers::types::{H256, U256};
    use ethers::abi::{encode, ParamType, Tokenizable};
    use ethers::types::transaction::eip712::{
        encode_eip712_type, make_type_hash, EIP712Domain,
    };
    use ethers::utils::keccak256;

    let cfg = config::load_hl_config()?;
    let wallet: ethers::signers::LocalWallet = cfg.private_key.parse().context("Invalid private key")?;
    let is_mainnet = !cfg.testnet;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    let amount_str = format!("{}", usdc);
    let chain = if is_mainnet { "Mainnet" } else { "Testnet" };
    let sig_chain_id = "0x66eee";

    let action = json!({
        "type": "usdClassTransfer",
        "hyperliquidChain": chain,
        "signatureChainId": sig_chain_id,
        "amount": amount_str,
        "toPerp": to_perp,
        "nonce": timestamp,
    });

    // EIP-712: sign UsdClassTransfer typed data
    // UsdClassTransfer(string hyperliquidChain,uint64 amount,bool toPerp,uint64 nonce)
    let type_hash = make_type_hash(
        "HyperliquidTransaction:UsdClassTransfer".to_string(),
        &[
            ("hyperliquidChain".to_string(), ParamType::String),
            ("amount".to_string(), ParamType::String),
            ("toPerp".to_string(), ParamType::Bool),
            ("nonce".to_string(), ParamType::Uint(64)),
        ],
    );

    let struct_items = vec![
        ethers::abi::Token::Uint(type_hash.into()),
        encode_eip712_type(chain.to_string().into_token()),
        encode_eip712_type(amount_str.clone().into_token()),
        encode_eip712_type(to_perp.into_token()),
        encode_eip712_type(ethers::abi::Token::Uint(U256::from(timestamp))),
    ];
    let struct_hash = keccak256(encode(&struct_items));

    let domain = EIP712Domain {
        name: Some("HyperliquidSignTransaction".to_string()),
        version: Some("1".to_string()),
        chain_id: Some(U256::from(421614u64)),
        verifying_contract: Some(
            "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
        ),
        salt: None,
    };

    let domain_separator = domain.separator();
    let digest = keccak256([
        &[0x19, 0x01],
        domain_separator.as_ref(),
        struct_hash.as_ref(),
    ].concat());

    let signature = wallet
        .sign_hash(H256::from(digest))
        .map_err(|e| anyhow::anyhow!("Signing failed: {:?}", e))?;

    let sig_json = json!({
        "r": format!("0x{:064x}", signature.r),
        "s": format!("0x{:064x}", signature.s),
        "v": signature.v,
    });

    let api_url = if is_mainnet {
        "https://api.hyperliquid.xyz/exchange"
    } else {
        "https://api.hyperliquid-testnet.xyz/exchange"
    };

    let payload = json!({
        "action": action,
        "nonce": timestamp,
        "signature": sig_json,
    });

    let http = reqwest::Client::builder()
        .user_agent("fintool/0.1")
        .build()?;

    let resp = http
        .post(api_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to send class transfer")?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response")?;

    if !status.is_success() {
        anyhow::bail!("Class transfer failed ({}): {}", status, text);
    }

    if !text.is_empty() {
        if let Ok(body) = serde_json::from_str::<Value>(&text) {
            if body.get("status").and_then(|s| s.as_str()) == Some("err") {
                let msg = body.get("response").and_then(|r| r.as_str()).unwrap_or("unknown error");
                anyhow::bail!("Class transfer rejected: {}", msg);
            }
        }
    }

    Ok(())
}

/// Enable unified account mode for HIP-3 dex margin sharing
pub async fn set_abstraction(mode: &str) -> Result<()> {
    use ethers::types::{H256, U256};
    use ethers::abi::{encode, ParamType, Tokenizable};
    use ethers::types::transaction::eip712::{
        encode_eip712_type, make_type_hash, EIP712Domain,
    };
    use ethers::utils::keccak256;

    let cfg = config::load_hl_config()?;
    let wallet: ethers::signers::LocalWallet = cfg.private_key.parse().context("Invalid private key")?;
    let is_mainnet = !cfg.testnet;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    let chain = if is_mainnet { "Mainnet" } else { "Testnet" };
    let sig_chain_id = "0x66eee";
    // Use the wallet's address (matches what the API recovers from the signature)
    let user_address = format!("{:?}", wallet.address()).to_lowercase();

    let action = json!({
        "type": "userSetAbstraction",
        "hyperliquidChain": chain,
        "signatureChainId": sig_chain_id,
        "user": user_address,
        "abstraction": mode,
        "nonce": timestamp,
    });

    // EIP-712: HyperliquidTransaction:UserSetAbstraction
    let type_hash = make_type_hash(
        "HyperliquidTransaction:UserSetAbstraction".to_string(),
        &[
            ("hyperliquidChain".to_string(), ParamType::String),
            ("user".to_string(), ParamType::Address),
            ("abstraction".to_string(), ParamType::String),
            ("nonce".to_string(), ParamType::Uint(64)),
        ],
    );

    let user_addr: ethers::types::Address = user_address.parse().context("Invalid address")?;

    let struct_items = vec![
        ethers::abi::Token::Uint(type_hash.into()),
        encode_eip712_type(chain.to_string().into_token()),
        encode_eip712_type(user_addr.into_token()),
        encode_eip712_type(mode.to_string().into_token()),
        encode_eip712_type(ethers::abi::Token::Uint(U256::from(timestamp))),
    ];
    let struct_hash = keccak256(encode(&struct_items));

    let domain = EIP712Domain {
        name: Some("HyperliquidSignTransaction".to_string()),
        version: Some("1".to_string()),
        chain_id: Some(U256::from(421614u64)),
        verifying_contract: Some(
            "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
        ),
        salt: None,
    };

    let domain_separator = domain.separator();
    let digest = keccak256([
        &[0x19, 0x01],
        domain_separator.as_ref(),
        struct_hash.as_ref(),
    ].concat());

    let signature = wallet
        .sign_hash(H256::from(digest))
        .map_err(|e| anyhow::anyhow!("Signing failed: {:?}", e))?;

    let sig_json = json!({
        "r": format!("0x{:064x}", signature.r),
        "s": format!("0x{:064x}", signature.s),
        "v": signature.v,
    });

    let api_url = if is_mainnet {
        "https://api.hyperliquid.xyz/exchange"
    } else {
        "https://api.hyperliquid-testnet.xyz/exchange"
    };

    let payload = json!({
        "action": action,
        "nonce": timestamp,
        "signature": sig_json,
    });

    let http = reqwest::Client::builder()
        .user_agent("fintool/0.1")
        .build()?;

    let resp = http
        .post(api_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to send set abstraction")?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response")?;

    if !status.is_success() {
        anyhow::bail!("Set abstraction failed ({}): {}", status, text);
    }

    if !text.is_empty() {
        if let Ok(body) = serde_json::from_str::<Value>(&text) {
            if body.get("status").and_then(|s| s.as_str()) == Some("err") {
                let msg = body.get("response").and_then(|r| r.as_str()).unwrap_or("unknown error");
                anyhow::bail!("Set abstraction rejected: {}", msg);
            }
        }
    }

    Ok(())
}

/// Look up the collateral token for a HIP-3 perp dex.
/// Returns (token_string, token_name) e.g. ("USDT0:0x25fa...", "USDT0")
/// For the main perp dex (dex=""), returns USDC.
pub async fn get_dex_collateral_token(dex: &str) -> Result<(String, String)> {
    let client = reqwest::Client::new();
    let url = config::info_url();

    // Main dex uses USDC (index 0)
    let collateral_idx: u64 = if dex.is_empty() {
        0
    } else {
        let meta: Value = client
            .post(&url)
            .json(&json!({"type": "meta", "dex": dex}))
            .send()
            .await?
            .json()
            .await
            .context("Failed to parse dex meta")?;

        meta.get("collateralToken")
            .and_then(|c| c.as_u64())
            .ok_or_else(|| anyhow::anyhow!("No collateralToken found for dex '{}'", dex))?
    };

    // Look up token details in spotMeta
    let spot_meta: Value = client
        .post(&url)
        .json(&json!({"type": "spotMeta"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse spotMeta")?;

    if let Some(tokens) = spot_meta.get("tokens").and_then(|t| t.as_array()) {
        for token in tokens {
            if token.get("index").and_then(|i| i.as_u64()) == Some(collateral_idx) {
                let name = token.get("name").and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Token {} has no name", collateral_idx))?;
                let token_id = token.get("tokenId").and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Token {} has no tokenId", collateral_idx))?;
                return Ok((format!("{}:{}", name, token_id), name.to_string()));
            }
        }
    }

    anyhow::bail!("Collateral token (index {}) not found in spotMeta for dex '{}'", collateral_idx, dex)
}

/// Transfer assets between dexes/spot using sendAsset
/// source_dex/destination_dex: "" for default perp, "spot" for spot, or dex name like "cash"
pub async fn send_asset(
    amount: f64,
    source_dex: &str,
    destination_dex: &str,
    token: &str,
) -> Result<()> {
    use ethers::types::{H256, U256};
    use ethers::abi::{encode, ParamType, Tokenizable};
    use ethers::types::transaction::eip712::{
        encode_eip712_type, make_type_hash, EIP712Domain,
    };
    use ethers::utils::keccak256;

    let cfg = config::load_hl_config()?;
    let wallet: ethers::signers::LocalWallet = cfg.private_key.parse().context("Invalid private key")?;
    let is_mainnet = !cfg.testnet;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    let chain = if is_mainnet { "Mainnet" } else { "Testnet" };
    let sig_chain_id = "0x66eee";
    // Use the wallet's address (matches what the API recovers from the signature)
    let destination = format!("{:?}", wallet.address()).to_lowercase();
    let amount_str = format!("{}", amount);

    let action = json!({
        "type": "sendAsset",
        "hyperliquidChain": chain,
        "signatureChainId": sig_chain_id,
        "destination": destination,
        "sourceDex": source_dex,
        "destinationDex": destination_dex,
        "token": token,
        "amount": amount_str,
        "fromSubAccount": "",
        "nonce": timestamp,
    });

    // EIP-712: HyperliquidTransaction:SendAsset
    let type_hash = make_type_hash(
        "HyperliquidTransaction:SendAsset".to_string(),
        &[
            ("hyperliquidChain".to_string(), ParamType::String),
            ("destination".to_string(), ParamType::String),
            ("sourceDex".to_string(), ParamType::String),
            ("destinationDex".to_string(), ParamType::String),
            ("token".to_string(), ParamType::String),
            ("amount".to_string(), ParamType::String),
            ("fromSubAccount".to_string(), ParamType::String),
            ("nonce".to_string(), ParamType::Uint(64)),
        ],
    );

    let struct_items = vec![
        ethers::abi::Token::Uint(type_hash.into()),
        encode_eip712_type(chain.to_string().into_token()),
        encode_eip712_type(destination.clone().into_token()),
        encode_eip712_type(source_dex.to_string().into_token()),
        encode_eip712_type(destination_dex.to_string().into_token()),
        encode_eip712_type(token.to_string().into_token()),
        encode_eip712_type(amount_str.clone().into_token()),
        encode_eip712_type("".to_string().into_token()),
        encode_eip712_type(ethers::abi::Token::Uint(U256::from(timestamp))),
    ];
    let struct_hash = keccak256(encode(&struct_items));

    let domain = EIP712Domain {
        name: Some("HyperliquidSignTransaction".to_string()),
        version: Some("1".to_string()),
        chain_id: Some(U256::from(421614u64)),
        verifying_contract: Some(
            "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
        ),
        salt: None,
    };

    let domain_separator = domain.separator();
    let digest = keccak256([
        &[0x19, 0x01],
        domain_separator.as_ref(),
        struct_hash.as_ref(),
    ].concat());

    let signature = wallet
        .sign_hash(H256::from(digest))
        .map_err(|e| anyhow::anyhow!("Signing failed: {:?}", e))?;

    let sig_json = json!({
        "r": format!("0x{:064x}", signature.r),
        "s": format!("0x{:064x}", signature.s),
        "v": signature.v,
    });

    let api_url = if is_mainnet {
        "https://api.hyperliquid.xyz/exchange"
    } else {
        "https://api.hyperliquid-testnet.xyz/exchange"
    };

    let payload = json!({
        "action": action,
        "nonce": timestamp,
        "signature": sig_json,
    });

    let http = reqwest::Client::builder()
        .user_agent("fintool/0.1")
        .build()?;

    let resp = http
        .post(api_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to send asset transfer")?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response")?;

    if !status.is_success() {
        anyhow::bail!("Send asset failed ({}): {}", status, text);
    }

    if !text.is_empty() {
        if let Ok(body) = serde_json::from_str::<Value>(&text) {
            if body.get("status").and_then(|s| s.as_str()) == Some("err") {
                let msg = body.get("response").and_then(|r| r.as_str()).unwrap_or("unknown error");
                anyhow::bail!("Send asset rejected: {}", msg);
            }
        }
    }

    Ok(())
}

/// Cancel an order by asset and order ID (works for both spot and perp)
pub async fn cancel_order(asset: &str, order_id: u64) -> Result<ExchangeResponseStatus> {
    // Try spot first, fall back to perp name
    let asset_name = resolve_spot_name(asset)
        .await
        .unwrap_or_else(|_| asset.to_string());

    let client = get_exchange_client().await?;

    let cancel = ClientCancelRequest {
        asset: asset_name,
        oid: order_id,
    };

    let result = client
        .cancel(cancel, None)
        .await
        .map_err(|e| anyhow::anyhow!("Cancel failed: {:?}", e))?;
    Ok(result)
}
