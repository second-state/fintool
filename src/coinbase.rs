use anyhow::{bail, Context, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::json;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const BASE_URL: &str = "https://api.coinbase.com";

/// Sign a request with HMAC-SHA256
/// timestamp + method + requestPath + body
pub fn sign_request(secret: &str, timestamp: &str, method: &str, path: &str, body: &str) -> String {
    let message = format!("{}{}{}{}", timestamp, method, path, body);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Get current timestamp in seconds
fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

/// Place a spot limit order on Coinbase
#[allow(clippy::too_many_arguments)]
pub async fn spot_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    side: &str, // "BUY" or "SELL"
    size: f64,
    price: f64,
    json_output: bool,
) -> Result<()> {
    let ts = timestamp();
    let path = "/api/v3/brokerage/orders";

    // Generate client_order_id
    let client_order_id = uuid::Uuid::new_v4().to_string();

    // Coinbase uses BTC-USD format (not BTCUSDT)
    let product_id = format!("{}-USD", symbol.to_uppercase());

    let body = json!({
        "client_order_id": client_order_id,
        "product_id": product_id,
        "side": side,
        "order_configuration": {
            "limit_limit_gtc": {
                "base_size": format!("{:.8}", size),
                "limit_price": format!("{:.2}", price),
            }
        }
    });

    let body_str = serde_json::to_string(&body)?;
    let signature = sign_request(api_secret, &ts, "POST", path, &body_str);

    let url = format!("{}{}", BASE_URL, path);

    let response = client
        .post(&url)
        .header("CB-ACCESS-KEY", api_key)
        .header("CB-ACCESS-SIGN", signature)
        .header("CB-ACCESS-TIMESTAMP", ts)
        .header("Content-Type", "application/json")
        .body(body_str)
        .send()
        .await
        .context("Failed to send Coinbase spot order")?;

    let status = response.status();
    let response_body: serde_json::Value =
        response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = response_body.get("message") {
            format!("Coinbase API error: {}", msg)
        } else if let Some(msg) = response_body.get("error") {
            format!("Coinbase API error: {}", msg)
        } else {
            format!("Coinbase API error: {:?}", response_body)
        };
        bail!(error_msg);
    }

    let result = json!({
        "exchange": "coinbase",
        "market": "spot",
        "action": side.to_lowercase(),
        "symbol": product_id,
        "quantity": size,
        "price": price,
        "response": response_body,
    });

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("\n  ✅ Coinbase spot {} order placed!", side.to_lowercase());
        println!(
            "  Order ID: {}",
            response_body.get("order_id").unwrap_or(&json!(null))
        );
        println!("  Product:  {}", product_id);
        println!("  Quantity: {:.8}", size);
        println!("  Price:    ${:.2}\n", price);
    }

    Ok(())
}

/// Get account balances
pub async fn get_accounts(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    json_output: bool,
) -> Result<()> {
    let ts = timestamp();
    let path = "/api/v3/brokerage/accounts";
    let signature = sign_request(api_secret, &ts, "GET", path, "");

    let url = format!("{}{}", BASE_URL, path);

    let response = client
        .get(&url)
        .header("CB-ACCESS-KEY", api_key)
        .header("CB-ACCESS-SIGN", signature)
        .header("CB-ACCESS-TIMESTAMP", ts)
        .send()
        .await
        .context("Failed to fetch Coinbase accounts")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("message") {
            format!("Coinbase API error: {}", msg)
        } else {
            format!("Coinbase API error: {:?}", body)
        };
        bail!(error_msg);
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "coinbase",
                "accounts": body,
            }))?
        );
        return Ok(());
    }

    println!("\n  💰 Coinbase Account Balance\n");

    if let Some(accounts) = body.get("accounts").and_then(|v| v.as_array()) {
        for account in accounts {
            let currency = account
                .get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let available_balance: f64 = account
                .get("available_balance")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let hold: f64 = account
                .get("hold")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);

            if available_balance > 0.0 || hold > 0.0 {
                use colored::Colorize;
                println!(
                    "  {}: {} (available: {}, hold: {})",
                    currency.cyan(),
                    available_balance + hold,
                    available_balance,
                    hold
                );
            }
        }
    }

    println!();
    Ok(())
}

/// Get open orders
pub async fn get_orders(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    symbol: Option<&str>,
    _json_output: bool,
) -> Result<serde_json::Value> {
    let ts = timestamp();
    let mut path = "/api/v3/brokerage/orders/historical/batch?order_status=OPEN".to_string();

    if let Some(sym) = symbol {
        let product_id = format!("{}-USD", sym.to_uppercase());
        path.push_str(&format!("&product_id={}", product_id));
    }

    let signature = sign_request(api_secret, &ts, "GET", &path, "");

    let url = format!("{}{}", BASE_URL, path);

    let response = client
        .get(&url)
        .header("CB-ACCESS-KEY", api_key)
        .header("CB-ACCESS-SIGN", signature)
        .header("CB-ACCESS-TIMESTAMP", ts)
        .send()
        .await
        .context("Failed to fetch Coinbase orders")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = body.get("message") {
            format!("Coinbase API error: {}", msg)
        } else {
            format!("Coinbase API error: {:?}", body)
        };
        bail!(error_msg);
    }

    Ok(body)
}

/// Cancel an order
pub async fn cancel_order(
    client: &Client,
    api_key: &str,
    api_secret: &str,
    order_id: &str,
    json_output: bool,
) -> Result<()> {
    let ts = timestamp();
    let path = "/api/v3/brokerage/orders/batch_cancel";

    let body = json!({
        "order_ids": [order_id]
    });

    let body_str = serde_json::to_string(&body)?;
    let signature = sign_request(api_secret, &ts, "POST", path, &body_str);

    let url = format!("{}{}", BASE_URL, path);

    let response = client
        .post(&url)
        .header("CB-ACCESS-KEY", api_key)
        .header("CB-ACCESS-SIGN", signature)
        .header("CB-ACCESS-TIMESTAMP", ts)
        .header("Content-Type", "application/json")
        .body(body_str)
        .send()
        .await
        .context("Failed to cancel Coinbase order")?;

    let status = response.status();
    let response_body: serde_json::Value =
        response.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let error_msg = if let Some(msg) = response_body.get("message") {
            format!("Coinbase API error: {}", msg)
        } else {
            format!("Coinbase API error: {:?}", response_body)
        };
        bail!(error_msg);
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "exchange": "coinbase",
                "order_id": order_id,
                "result": response_body,
            }))?
        );
    } else {
        use colored::Colorize;
        println!("\n  ✅ Coinbase order cancelled!");
        println!("  Order ID: {}\n", order_id.cyan());
    }

    Ok(())
}
