use anyhow::{bail, Context, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::json;
use sha2::Sha256;

use crate::config;

type HmacSha256 = Hmac<Sha256>;

/// Get the OKX base URL from config (customizable for OKX US / EEA)
fn base_url() -> String {
    config::okx_base_url()
}

/// Generate ISO 8601 timestamp for OKX API
fn iso_timestamp() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// Sign a request with HMAC-SHA256, base64-encoded
/// payload = timestamp + METHOD + requestPath + body
fn sign_request(secret: &str, timestamp: &str, method: &str, path: &str, body: &str) -> String {
    let payload = format!("{}{}{}{}", timestamp, method, path, body);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(result.into_bytes())
}

/// Make an authenticated GET request to OKX API
async fn okx_get(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    path: &str,
) -> Result<serde_json::Value> {
    let timestamp = iso_timestamp();
    let signature = sign_request(api_secret, &timestamp, "GET", path, "");
    let url = format!("{}{}", base_url(), path);

    let response = client
        .get(&url)
        .header("OK-ACCESS-KEY", api_key)
        .header("OK-ACCESS-SIGN", &signature)
        .header("OK-ACCESS-PASSPHRASE", passphrase)
        .header("OK-ACCESS-TIMESTAMP", &timestamp)
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to call OKX API")?;

    let body: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse OKX response")?;
    check_okx_response(&body)?;
    Ok(body)
}

/// Make an authenticated POST request to OKX API
async fn okx_post(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    path: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value> {
    let timestamp = iso_timestamp();
    let body_str = serde_json::to_string(body)?;
    let signature = sign_request(api_secret, &timestamp, "POST", path, &body_str);
    let url = format!("{}{}", base_url(), path);

    let response = client
        .post(&url)
        .header("OK-ACCESS-KEY", api_key)
        .header("OK-ACCESS-SIGN", &signature)
        .header("OK-ACCESS-PASSPHRASE", passphrase)
        .header("OK-ACCESS-TIMESTAMP", &timestamp)
        .header("Content-Type", "application/json")
        .body(body_str)
        .send()
        .await
        .context("Failed to call OKX API")?;

    let resp: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse OKX response")?;
    check_okx_response(&resp)?;
    Ok(resp)
}

/// Make a public (unauthenticated) GET request
async fn okx_public_get(client: &Client, path: &str) -> Result<serde_json::Value> {
    let url = format!("{}{}", base_url(), path);
    let response = client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to call OKX API")?;

    let body: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse OKX response")?;
    check_okx_response(&body)?;
    Ok(body)
}

/// Check OKX API response for errors (code != "0")
fn check_okx_response(resp: &serde_json::Value) -> Result<()> {
    let code = resp["code"].as_str().unwrap_or("0");
    if code != "0" {
        let msg = resp["msg"].as_str().unwrap_or("Unknown error");
        bail!("OKX API error (code {}): {}", code, msg);
    }
    Ok(())
}

// ── Public Market Endpoints (no auth) ────────────────────────────────

/// Get ticker for an instrument (no auth)
pub async fn get_ticker(client: &Client, inst_id: &str) -> Result<serde_json::Value> {
    let path = format!("/api/v5/market/ticker?instId={}", inst_id);
    okx_public_get(client, &path).await
}

/// Get order book (no auth)
pub async fn get_orderbook(
    client: &Client,
    inst_id: &str,
    depth: usize,
) -> Result<serde_json::Value> {
    let path = format!("/api/v5/market/books?instId={}&sz={}", inst_id, depth);
    okx_public_get(client, &path).await
}

/// Get funding rate for a swap instrument (no auth)
pub async fn get_funding_rate(client: &Client, inst_id: &str) -> Result<serde_json::Value> {
    let path = format!("/api/v5/public/funding-rate?instId={}", inst_id);
    okx_public_get(client, &path).await
}

/// Get mark price (no auth)
pub async fn get_mark_price(
    client: &Client,
    inst_type: &str,
    inst_id: Option<&str>,
) -> Result<serde_json::Value> {
    let mut path = format!("/api/v5/public/mark-price?instType={}", inst_type);
    if let Some(id) = inst_id {
        path = format!("{}&instId={}", path, id);
    }
    okx_public_get(client, &path).await
}

// ── Account Endpoints (auth required) ────────────────────────────────

/// Get trading account balance
pub async fn get_balance(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
) -> Result<serde_json::Value> {
    okx_get(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/account/balance",
    )
    .await
}

/// Get funding account balance
pub async fn get_funding_balance(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
) -> Result<serde_json::Value> {
    okx_get(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/asset/balances",
    )
    .await
}

/// Get open positions
pub async fn get_positions(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
) -> Result<serde_json::Value> {
    okx_get(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/account/positions",
    )
    .await
}

/// Get open/pending orders
pub async fn get_pending_orders(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    inst_type: Option<&str>,
) -> Result<serde_json::Value> {
    let path = if let Some(t) = inst_type {
        format!("/api/v5/trade/orders-pending?instType={}", t)
    } else {
        "/api/v5/trade/orders-pending".to_string()
    };
    okx_get(client, api_key, api_secret, passphrase, &path).await
}

// ── Trading Endpoints ────────────────────────────────────────────────

/// Place a single order
/// td_mode: "cash" (spot), "cross" (cross margin), "isolated"
#[allow(clippy::too_many_arguments)]
pub async fn place_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    inst_id: &str,
    td_mode: &str,
    side: &str,
    ord_type: &str,
    size: &str,
    price: Option<&str>,
    reduce_only: bool,
) -> Result<serde_json::Value> {
    let mut body = json!({
        "instId": inst_id,
        "tdMode": td_mode,
        "side": side,
        "ordType": ord_type,
        "sz": size,
    });
    if let Some(px) = price {
        body["px"] = json!(px);
    }
    if reduce_only {
        body["reduceOnly"] = json!(true);
    }

    okx_post(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/trade/order",
        &body,
    )
    .await
}

/// Cancel an order
pub async fn cancel_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    inst_id: &str,
    ord_id: &str,
) -> Result<serde_json::Value> {
    let body = json!({
        "instId": inst_id,
        "ordId": ord_id,
    });
    okx_post(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/trade/cancel-order",
        &body,
    )
    .await
}

/// Set leverage for an instrument
pub async fn set_leverage(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    inst_id: &str,
    leverage: u32,
    mgn_mode: &str, // "cross" or "isolated"
) -> Result<serde_json::Value> {
    let body = json!({
        "instId": inst_id,
        "lever": leverage.to_string(),
        "mgnMode": mgn_mode,
    });
    okx_post(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/account/set-leverage",
        &body,
    )
    .await
}

// ── Asset / Wallet Endpoints ─────────────────────────────────────────

/// Get deposit address for a currency
pub async fn get_deposit_address(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    ccy: &str,
) -> Result<serde_json::Value> {
    let path = format!("/api/v5/asset/deposit-address?ccy={}", ccy);
    okx_get(client, api_key, api_secret, passphrase, &path).await
}

/// Submit a withdrawal
#[allow(clippy::too_many_arguments)]
pub async fn withdraw(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    ccy: &str,
    amount: &str,
    dest: &str,    // "3" = internal, "4" = on-chain
    to_addr: &str, // destination address
    chain: &str,   // e.g. "USDC-Base", "ETH-ERC20"
    fee: &str,     // withdrawal fee
) -> Result<serde_json::Value> {
    let body = json!({
        "ccy": ccy,
        "amt": amount,
        "dest": dest,
        "toAddr": to_addr,
        "chain": chain,
        "fee": fee,
    });
    okx_post(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/asset/withdrawal",
        &body,
    )
    .await
}

/// Transfer between accounts (funding ↔ trading)
/// from/to: "6" = funding, "18" = trading (unified)
#[allow(clippy::too_many_arguments)]
pub async fn transfer(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    ccy: &str,
    amount: &str,
    from: &str,
    to: &str,
) -> Result<serde_json::Value> {
    let body = json!({
        "ccy": ccy,
        "amt": amount,
        "from": from,
        "to": to,
        "type": "0",  // within main account
    });
    okx_post(
        client,
        api_key,
        api_secret,
        passphrase,
        "/api/v5/asset/transfer",
        &body,
    )
    .await
}

/// Get withdrawal fee for a currency
pub async fn get_currencies(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    ccy: Option<&str>,
) -> Result<serde_json::Value> {
    let path = if let Some(c) = ccy {
        format!("/api/v5/asset/currencies?ccy={}", c)
    } else {
        "/api/v5/asset/currencies".to_string()
    };
    okx_get(client, api_key, api_secret, passphrase, &path).await
}

// ── Helper: OKX chain name mapping ───────────────────────────────────

/// Map user-friendly network names to OKX chain identifiers
pub fn map_chain(ccy: &str, network: &str) -> String {
    let ccy_upper = ccy.to_uppercase();
    match network.to_lowercase().as_str() {
        "ethereum" | "eth" | "erc20" => format!("{}-ERC20", ccy_upper),
        "base" => format!("{}-Base", ccy_upper),
        "arbitrum" | "arb" => format!("{}-Arbitrum One", ccy_upper),
        "optimism" | "op" => format!("{}-Optimism", ccy_upper),
        "polygon" | "matic" => format!("{}-Polygon", ccy_upper),
        "solana" | "sol" => format!("{}-Solana", ccy_upper),
        "bitcoin" | "btc" => format!("{}-Bitcoin", ccy_upper),
        "bsc" | "bnb" => format!("{}-BSC", ccy_upper),
        "avalanche" | "avax" => format!("{}-Avalanche C-Chain", ccy_upper),
        other => format!("{}-{}", ccy_upper, other),
    }
}

// ── Helper: OKX instrument ID formatting ─────────────────────────────

/// Format a spot instrument ID: BTC -> BTC-USDT
pub fn spot_inst_id(symbol: &str) -> String {
    format!("{}-USDT", symbol.to_uppercase())
}

/// Format a swap (perp) instrument ID: BTC -> BTC-USDT-SWAP
pub fn swap_inst_id(symbol: &str) -> String {
    format!("{}-USDT-SWAP", symbol.to_uppercase())
}
