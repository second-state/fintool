use anyhow::{Result, Context};
use ethers::signers::LocalWallet;
use hyperliquid_rust_sdk::{
    BaseUrl, ExchangeClient, ExchangeResponseStatus,
    ClientOrderRequest, ClientOrder, ClientLimit, ClientCancelRequest,
};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::config;

/// Create a LocalWallet from config
pub fn get_wallet() -> Result<LocalWallet> {
    let cfg = config::load_hl_config()?;
    let wallet: LocalWallet = cfg.private_key.parse()
        .context("Failed to parse private key")?;
    Ok(wallet)
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

/// Place a spot limit order
pub async fn place_spot_order(
    symbol: &str,
    is_buy: bool,
    price: f64,
    size: f64,
) -> Result<ExchangeResponseStatus> {
    let asset_name = resolve_spot_name(symbol).await?;
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

    let result = client.order(order, None)
        .await
        .map_err(|e| anyhow::anyhow!("Spot order failed: {:?}", e))?;
    Ok(result)
}

/// Place a perp limit order
pub async fn place_perp_order(
    symbol: &str,
    is_buy: bool,
    price: f64,
    size: f64,
) -> Result<ExchangeResponseStatus> {
    let client = get_exchange_client().await?;

    let order = ClientOrderRequest {
        asset: symbol.to_string(),
        is_buy,
        reduce_only: false,
        limit_px: price,
        sz: size,
        cloid: None,
        order_type: ClientOrder::Limit(ClientLimit {
            tif: "Gtc".to_string(),
        }),
    };

    let result = client.order(order, None)
        .await
        .map_err(|e| anyhow::anyhow!("Perp order failed: {:?}", e))?;
    Ok(result)
}

/// Cancel an order by asset and order ID (works for both spot and perp)
pub async fn cancel_order(asset: &str, order_id: u64) -> Result<ExchangeResponseStatus> {
    // Try spot first, fall back to perp name
    let asset_name = resolve_spot_name(asset).await
        .unwrap_or_else(|_| asset.to_string());

    let client = get_exchange_client().await?;

    let cancel = ClientCancelRequest {
        asset: asset_name,
        oid: order_id,
    };

    let result = client.cancel(cancel, None)
        .await
        .map_err(|e| anyhow::anyhow!("Cancel failed: {:?}", e))?;
    Ok(result)
}
