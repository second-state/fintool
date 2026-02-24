use anyhow::{bail, Context, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::json;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const SPOT_BASE_URL: &str = "https://api.binance.com";
const FUTURES_BASE_URL: &str = "https://fapi.binance.com";
const OPTIONS_BASE_URL: &str = "https://eapi.binance.com";

/// Sign a request with HMAC-SHA256
pub fn sign_request(secret: &str, query_string: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(query_string.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Get current timestamp in milliseconds
fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Place a spot limit order on Binance
#[allow(clippy::too_many_arguments)]
pub async fn spot_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    side: &str, // "BUY" or "SELL"
    qty: f64,
    price: f64,
    json_output: bool,
) -> Result<()> {
    let timestamp = timestamp_ms();
    let query_string = format!(
        "symbol={}&side={}&type=LIMIT&timeInForce=GTC&quantity={:.8}&price={:.8}&timestamp={}",
        symbol, side, qty, price, timestamp
    );
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/api/v3/order?{}&signature={}",
        SPOT_BASE_URL, query_string, signature
    );

    let response = client
        .post(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to send Binance spot order")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    let result = json!({
        "exchange": "binance",
        "market": "spot",
        "action": side.to_lowercase(),
        "symbol": symbol,
        "quantity": qty,
        "price": price,
        "response": body,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("\n  ✅ Binance spot {} order placed!", side.to_lowercase());
        println!(
            "  Order ID: {}",
            body.get("orderId").unwrap_or(&json!(null))
        );
        println!("  Symbol:   {}", symbol);
        println!("  Quantity: {:.8}", qty);
        println!("  Price:    ${:.8}\n", price);
    }

    Ok(())
}

/// Place a futures limit order on Binance
#[allow(clippy::too_many_arguments)]
pub async fn futures_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    side: &str, // "BUY" or "SELL"
    qty: f64,
    price: f64,
    json_output: bool,
) -> Result<()> {
    let timestamp = timestamp_ms();
    let query_string = format!(
        "symbol={}&side={}&type=LIMIT&timeInForce=GTC&quantity={:.8}&price={:.8}&timestamp={}",
        symbol, side, qty, price, timestamp
    );
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/fapi/v1/order?{}&signature={}",
        FUTURES_BASE_URL, query_string, signature
    );

    let response = client
        .post(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to send Binance futures order")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    let result = json!({
        "exchange": "binance",
        "market": "futures",
        "action": side.to_lowercase(),
        "symbol": symbol,
        "quantity": qty,
        "price": price,
        "response": body,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "\n  ✅ Binance futures {} order placed!",
            side.to_lowercase()
        );
        println!(
            "  Order ID: {}",
            body.get("orderId").unwrap_or(&json!(null))
        );
        println!("  Symbol:   {}", symbol);
        println!("  Quantity: {:.8}", qty);
        println!("  Price:    ${:.8}\n", price);
    }

    Ok(())
}

/// Place an options order on Binance
/// Symbol format: BTC-260328-80000-C (BASE-YYMMDD-STRIKE-C/P)
#[allow(clippy::too_many_arguments)]
pub async fn options_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    side: &str,        // "BUY" or "SELL"
    option_type: &str, // "call" or "put"
    strike: f64,
    expiry: &str, // YYMMDD format
    qty: f64,
    json_output: bool,
) -> Result<()> {
    // Build Binance options symbol: BASE-YYMMDD-STRIKE-C/P
    let base = symbol.to_uppercase();
    let option_char = if option_type.to_lowercase() == "call" {
        "C"
    } else {
        "P"
    };
    let binance_symbol = format!("{}-{}-{}-{}", base, expiry, strike as u64, option_char);

    let timestamp = timestamp_ms();
    // For options, we need a price - for now, use market order or require price parameter
    // This is simplified - in production you'd want to get market price or require it as param
    let query_string = format!(
        "symbol={}&side={}&type=MARKET&quantity={:.8}&timestamp={}",
        binance_symbol, side, qty, timestamp
    );
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/eapi/v1/order?{}&signature={}",
        OPTIONS_BASE_URL, query_string, signature
    );

    let response = client
        .post(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to send Binance options order")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    let result = json!({
        "exchange": "binance",
        "market": "options",
        "action": side.to_lowercase(),
        "symbol": binance_symbol,
        "option_type": option_type,
        "strike": strike,
        "expiry": expiry,
        "quantity": qty,
        "response": body,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "\n  ✅ Binance options {} order placed!",
            side.to_lowercase()
        );
        println!(
            "  Order ID: {}",
            body.get("orderId").unwrap_or(&json!(null))
        );
        println!("  Symbol:   {}", binance_symbol);
        println!("  Type:     {} {}", option_type, side.to_lowercase());
        println!("  Quantity: {:.8}\n", qty);
    }

    Ok(())
}

/// Get spot account balances
pub async fn get_spot_balances(
    client: &Client,
    api_key: &str,
    api_secret: &str,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let query_string = format!("timestamp={}", timestamp);
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/api/v3/account?{}&signature={}",
        SPOT_BASE_URL, query_string, signature
    );

    let response = client
        .get(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to fetch Binance spot balances")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Get futures account balances
pub async fn get_futures_balances(
    client: &Client,
    api_key: &str,
    api_secret: &str,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let query_string = format!("timestamp={}", timestamp);
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/fapi/v2/balance?{}&signature={}",
        FUTURES_BASE_URL, query_string, signature
    );

    let response = client
        .get(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to fetch Binance futures balances")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Get futures positions
pub async fn get_futures_positions(
    client: &Client,
    api_key: &str,
    api_secret: &str,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let query_string = format!("timestamp={}", timestamp);
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/fapi/v2/positionRisk?{}&signature={}",
        FUTURES_BASE_URL, query_string, signature
    );

    let response = client
        .get(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to fetch Binance futures positions")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Get open spot orders
pub async fn get_spot_open_orders(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: Option<&str>,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let query_string = if let Some(sym) = symbol {
        format!("symbol={}&timestamp={}", sym, timestamp)
    } else {
        format!("timestamp={}", timestamp)
    };
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/api/v3/openOrders?{}&signature={}",
        SPOT_BASE_URL, query_string, signature
    );

    let response = client
        .get(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to fetch Binance spot open orders")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Get open futures orders
pub async fn get_futures_open_orders(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: Option<&str>,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let query_string = if let Some(sym) = symbol {
        format!("symbol={}&timestamp={}", sym, timestamp)
    } else {
        format!("timestamp={}", timestamp)
    };
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/fapi/v1/openOrders?{}&signature={}",
        FUTURES_BASE_URL, query_string, signature
    );

    let response = client
        .get(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to fetch Binance futures open orders")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Cancel a spot order
pub async fn cancel_spot_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    order_id: u64,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let query_string = format!(
        "symbol={}&orderId={}&timestamp={}",
        symbol, order_id, timestamp
    );
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/api/v3/order?{}&signature={}",
        SPOT_BASE_URL, query_string, signature
    );

    let response = client
        .delete(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to cancel Binance spot order")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Get deposit address for a coin on a specific network
pub async fn get_deposit_address(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    coin: &str,
    network: Option<&str>,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let mut query_string = format!("coin={}&timestamp={}", coin.to_uppercase(), timestamp);
    if let Some(net) = network {
        query_string = format!("{}&network={}", query_string, net);
    }
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/sapi/v1/capital/deposit/address?{}&signature={}",
        SPOT_BASE_URL, query_string, signature
    );

    let response = client
        .get(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to get Binance deposit address")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Submit a withdrawal request
pub async fn withdraw(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    coin: &str,
    address: &str,
    amount: &str,
    network: Option<&str>,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let mut query_string = format!(
        "coin={}&address={}&amount={}&timestamp={}",
        coin.to_uppercase(),
        address,
        amount,
        timestamp
    );
    if let Some(net) = network {
        query_string = format!("{}&network={}", query_string, net);
    }
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/sapi/v1/capital/withdraw/apply?{}&signature={}",
        SPOT_BASE_URL, query_string, signature
    );

    let response = client
        .post(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to submit Binance withdrawal")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Cancel a futures order
pub async fn cancel_futures_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    order_id: u64,
) -> Result<serde_json::Value> {
    let timestamp = timestamp_ms();
    let query_string = format!(
        "symbol={}&orderId={}&timestamp={}",
        symbol, order_id, timestamp
    );
    let signature = sign_request(api_secret, &query_string);
    let url = format!(
        "{}/fapi/v1/order?{}&signature={}",
        FUTURES_BASE_URL, query_string, signature
    );

    let response = client
        .delete(&url)
        .header("X-MBX-APIKEY", api_key)
        .send()
        .await
        .context("Failed to cancel Binance futures order")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("msg") {
            format!("Binance API error: {}", msg)
        } else {
            format!("Binance API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}
