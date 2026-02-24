use anyhow::{bail, Context, Result};
use ethers::core::types::{Address, U256};
use ethers::signers::{LocalWallet, Signer};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use std::str::FromStr;

type HmacSha256 = Hmac<Sha256>;

const CLOB_BASE: &str = "https://clob.polymarket.com";
const GAMMA_BASE: &str = "https://gamma-api.polymarket.com";
const CTF_EXCHANGE: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
const NEG_RISK_CTF_EXCHANGE: &str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";

// --- Credentials cache ---

#[derive(Debug, Serialize, Deserialize)]
struct CredentialCache {
    address: String,
    api_key: String,
    secret: String,
    passphrase: String,
}

fn creds_cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".fintool")
        .join("polymarket_creds.json")
}

fn load_cached_credentials(address: &str) -> Option<(String, String, String)> {
    let path = creds_cache_path();
    if !path.exists() {
        return None;
    }
    let data = std::fs::read_to_string(&path).ok()?;
    let cache: CredentialCache = serde_json::from_str(&data).ok()?;
    if cache.address.eq_ignore_ascii_case(address) {
        Some((cache.api_key, cache.secret, cache.passphrase))
    } else {
        None
    }
}

fn save_cached_credentials(
    address: &str,
    api_key: &str,
    secret: &str,
    passphrase: &str,
) -> Result<()> {
    let path = creds_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cache = CredentialCache {
        address: address.to_string(),
        api_key: api_key.to_string(),
        secret: secret.to_string(),
        passphrase: passphrase.to_string(),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

// --- L1 Auth (API credential derivation) ---

fn clob_auth_eip712_hash(address: &Address, timestamp: &str) -> Result<[u8; 32]> {
    // Domain separator
    let domain_type_hash =
        ethers::utils::keccak256("EIP712Domain(string name,string version,uint256 chainId)");
    let name_hash = ethers::utils::keccak256("ClobAuthDomain");
    let version_hash = ethers::utils::keccak256("1");
    let chain_id = U256::from(137);

    let domain_separator = ethers::utils::keccak256(ethers::abi::encode(&[
        ethers::abi::Token::FixedBytes(domain_type_hash.to_vec()),
        ethers::abi::Token::FixedBytes(name_hash.to_vec()),
        ethers::abi::Token::FixedBytes(version_hash.to_vec()),
        ethers::abi::Token::Uint(chain_id),
    ]));

    // Struct hash
    let type_hash = ethers::utils::keccak256(
        "ClobAuth(address address,string timestamp,uint256 nonce,string message)",
    );
    let timestamp_hash = ethers::utils::keccak256(timestamp.as_bytes());
    let message_hash =
        ethers::utils::keccak256("This message attests that I control the given wallet".as_bytes());

    let struct_hash = ethers::utils::keccak256(ethers::abi::encode(&[
        ethers::abi::Token::FixedBytes(type_hash.to_vec()),
        ethers::abi::Token::Address(*address),
        ethers::abi::Token::FixedBytes(timestamp_hash.to_vec()),
        ethers::abi::Token::Uint(U256::zero()),
        ethers::abi::Token::FixedBytes(message_hash.to_vec()),
    ]));

    // Final digest
    let mut prefix = vec![0x19, 0x01];
    prefix.extend_from_slice(&domain_separator);
    prefix.extend_from_slice(&struct_hash);
    let digest = ethers::utils::keccak256(&prefix);

    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    Ok(result)
}

/// Derive API credentials (or use cached ones)
pub async fn derive_api_credentials(
    client: &reqwest::Client,
    private_key: &str,
) -> Result<(String, String, String)> {
    let wallet: LocalWallet = private_key
        .parse()
        .context("Invalid private key for Polymarket")?;
    let address = ethers::utils::to_checksum(&wallet.address(), None);

    // Check cache first
    if let Some((api_key, secret, passphrase)) = load_cached_credentials(&address) {
        return Ok((api_key, secret, passphrase));
    }

    // L1 auth: sign EIP-712 message
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .to_string();

    let hash = clob_auth_eip712_hash(&wallet.address(), &timestamp)?;
    let signature = wallet.sign_hash(hash.into())?;
    let sig_hex = format!("0x{}", hex::encode(signature.to_vec()));

    // Try to create API key first (for new wallets), fall back to derive
    let create_url = format!("{}/auth/api-key", CLOB_BASE);
    let resp = client
        .post(&create_url)
        .header("POLY_ADDRESS", &address)
        .header("POLY_SIGNATURE", &sig_hex)
        .header("POLY_TIMESTAMP", &timestamp)
        .header("POLY_NONCE", "0")
        .send()
        .await?;

    if resp.status().is_success() {
        let body: serde_json::Value = resp.json().await?;
        let api_key = body["apiKey"].as_str().unwrap_or_default().to_string();
        let secret = body["secret"].as_str().unwrap_or_default().to_string();
        let passphrase = body["passphrase"].as_str().unwrap_or_default().to_string();
        if !api_key.is_empty() {
            let _ = save_cached_credentials(&address, &api_key, &secret, &passphrase);
            return Ok((api_key, secret, passphrase));
        }
    }

    // Fall back to derive existing credentials
    let derive_url = format!("{}/auth/derive-api-key", CLOB_BASE);
    let resp = client
        .get(&derive_url)
        .header("POLY_ADDRESS", &address)
        .header("POLY_SIGNATURE", &sig_hex)
        .header("POLY_TIMESTAMP", &timestamp)
        .header("POLY_NONCE", "0")
        .send()
        .await?;

    if !resp.status().is_success() {
        bail!(
            "Failed to derive API credentials: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }

    #[derive(Deserialize)]
    struct ApiKeyResponse {
        #[serde(rename = "apiKey")]
        api_key: String,
        secret: String,
        passphrase: String,
    }

    let creds: ApiKeyResponse = resp.json().await?;

    // Cache credentials
    save_cached_credentials(&address, &creds.api_key, &creds.secret, &creds.passphrase)?;

    Ok((creds.api_key, creds.secret, creds.passphrase))
}

// --- L2 Auth (HMAC request signing) ---

fn sign_l2_request(secret: &str, timestamp: &str, method: &str, path: &str, body: &str) -> String {
    let message = format!("{}{}{}{}", timestamp, method, path, body);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

// --- Order Signing (EIP-712) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
    pub salt: String,
    pub maker: String,
    pub signer: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: String,
    pub taker_amount: String,
    pub expiration: String,
    pub nonce: String,
    pub fee_rate_bps: String,
    pub side: u8,
    pub signature_type: u8,
}

fn order_eip712_hash(order_data: &OrderData, exchange_address: &str) -> Result<[u8; 32]> {
    let salt = U256::from_dec_str(&order_data.salt)?;
    let maker = Address::from_str(&order_data.maker)?;
    let signer = Address::from_str(&order_data.signer)?;
    let taker = Address::from_str(&order_data.taker)?;
    let token_id = U256::from_dec_str(&order_data.token_id)?;
    let maker_amount = U256::from_dec_str(&order_data.maker_amount)?;
    let taker_amount = U256::from_dec_str(&order_data.taker_amount)?;
    let expiration = U256::from_dec_str(&order_data.expiration)?;
    let nonce = U256::from_dec_str(&order_data.nonce)?;
    let fee_rate_bps = U256::from_dec_str(&order_data.fee_rate_bps)?;
    let side = order_data.side;
    let signature_type = order_data.signature_type;
    // Domain separator
    let domain_type_hash = ethers::utils::keccak256(
        "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = ethers::utils::keccak256("Polymarket CTF Exchange");
    let version_hash = ethers::utils::keccak256("1");
    let chain_id = U256::from(137);
    let verifying_contract = Address::from_str(exchange_address)?;

    let domain_separator = ethers::utils::keccak256(ethers::abi::encode(&[
        ethers::abi::Token::FixedBytes(domain_type_hash.to_vec()),
        ethers::abi::Token::FixedBytes(name_hash.to_vec()),
        ethers::abi::Token::FixedBytes(version_hash.to_vec()),
        ethers::abi::Token::Uint(chain_id),
        ethers::abi::Token::Address(verifying_contract),
    ]));

    // Struct hash
    let type_hash = ethers::utils::keccak256(
        "Order(uint256 salt,address maker,address signer,address taker,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint256 expiration,uint256 nonce,uint256 feeRateBps,uint8 side,uint8 signatureType)"
    );

    let struct_hash = ethers::utils::keccak256(ethers::abi::encode(&[
        ethers::abi::Token::FixedBytes(type_hash.to_vec()),
        ethers::abi::Token::Uint(salt),
        ethers::abi::Token::Address(maker),
        ethers::abi::Token::Address(signer),
        ethers::abi::Token::Address(taker),
        ethers::abi::Token::Uint(token_id),
        ethers::abi::Token::Uint(maker_amount),
        ethers::abi::Token::Uint(taker_amount),
        ethers::abi::Token::Uint(expiration),
        ethers::abi::Token::Uint(nonce),
        ethers::abi::Token::Uint(fee_rate_bps),
        ethers::abi::Token::Uint(U256::from(side)),
        ethers::abi::Token::Uint(U256::from(signature_type)),
    ]));

    // Final digest
    let mut prefix = vec![0x19, 0x01];
    prefix.extend_from_slice(&domain_separator);
    prefix.extend_from_slice(&struct_hash);
    let digest = ethers::utils::keccak256(&prefix);

    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    Ok(result)
}

pub async fn sign_order(
    private_key: &str,
    order_data: &OrderData,
    neg_risk: bool,
) -> Result<String> {
    let wallet: LocalWallet = private_key.parse()?;
    let exchange = if neg_risk {
        NEG_RISK_CTF_EXCHANGE
    } else {
        CTF_EXCHANGE
    };

    let hash = order_eip712_hash(order_data, exchange)?;
    let signature = wallet.sign_hash(hash.into())?;
    Ok(format!("0x{}", hex::encode(signature.to_vec())))
}

// --- Market data fetching ---

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GammaMarket {
    #[serde(rename = "clobTokenIds")]
    clob_token_ids: Option<serde_json::Value>,
    slug: Option<String>,
    #[serde(rename = "negRisk")]
    neg_risk: Option<bool>,
}

/// Fetch token IDs and neg_risk flag for a market slug
pub async fn get_market_info(client: &reqwest::Client, slug: &str) -> Result<(Vec<String>, bool)> {
    let url = format!("{}/markets?slug={}", GAMMA_BASE, urlencoding::encode(slug));
    let markets: Vec<GammaMarket> = client.get(&url).send().await?.json().await?;
    let market = markets.first().context("Market not found")?;
    let neg_risk = market.neg_risk.unwrap_or(false);

    // clobTokenIds can be a JSON array or a JSON string containing an array
    let token_ids = match &market.clob_token_ids {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(serde_json::Value::String(s)) => {
            serde_json::from_str::<Vec<String>>(s).unwrap_or_default()
        }
        _ => Vec::new(),
    };
    if token_ids.is_empty() {
        bail!("No CLOB token IDs found for market");
    }
    Ok((token_ids, neg_risk))
}

/// Get tick size for a token
pub async fn get_tick_size(client: &reqwest::Client, token_id: &str) -> Result<f64> {
    let url = format!("{}/tick-size?token_id={}", CLOB_BASE, token_id);
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    // Handle both "minimum_tick_size" (number) and "tickSize" (string)
    resp.get("minimum_tick_size")
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .or_else(|| {
            resp.get("tickSize").and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
        })
        .context("Failed to get tick size")
}

/// Round price to tick size
pub fn round_to_tick(price: f64, tick_size: f64) -> f64 {
    if tick_size <= 0.0 {
        return price;
    }
    (price / tick_size).round() * tick_size
}

// --- Order submission ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderRequest {
    order: OrderWithSignature,
    owner: String,
    order_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderWithSignature {
    salt: String,
    maker: String,
    signer: String,
    taker: String,
    token_id: String,
    maker_amount: String,
    taker_amount: String,
    expiration: String,
    nonce: String,
    fee_rate_bps: String,
    side: u8,
    signature_type: u8,
    signature: String,
}

#[derive(Debug, Deserialize)]
pub struct OrderResponse {
    #[serde(rename = "orderID")]
    pub order_id: Option<String>,
    pub success: Option<bool>,
    pub error: Option<String>,
}

pub async fn post_order(
    client: &reqwest::Client,
    api_key: &str,
    secret: &str,
    passphrase: &str,
    address: &str,
    order_data: &OrderData,
    signature: &str,
) -> Result<OrderResponse> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .to_string();

    let request = OrderRequest {
        order: OrderWithSignature {
            salt: order_data.salt.clone(),
            maker: order_data.maker.clone(),
            signer: order_data.signer.clone(),
            taker: order_data.taker.clone(),
            token_id: order_data.token_id.clone(),
            maker_amount: order_data.maker_amount.clone(),
            taker_amount: order_data.taker_amount.clone(),
            expiration: order_data.expiration.clone(),
            nonce: order_data.nonce.clone(),
            fee_rate_bps: order_data.fee_rate_bps.clone(),
            side: order_data.side,
            signature_type: order_data.signature_type,
            signature: signature.to_string(),
        },
        owner: address.to_string(),
        order_type: "GTC".to_string(),
    };

    let body = serde_json::to_string(&request)?;
    let path = "/order";
    let l2_sig = sign_l2_request(secret, &timestamp, "POST", path, &body);

    let url = format!("{}{}", CLOB_BASE, path);
    let resp = client
        .post(&url)
        .header("POLY_ADDRESS", address)
        .header("POLY_SIGNATURE", &l2_sig)
        .header("POLY_TIMESTAMP", &timestamp)
        .header("POLY_NONCE", "0")
        .header("POLY_API_KEY", api_key)
        .header("POLY_PASSPHRASE", passphrase)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        bail!(
            "Order submission failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }

    let order_resp: OrderResponse = resp.json().await?;
    Ok(order_resp)
}

// --- Open orders ---

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct OpenOrder {
    pub id: String,
    #[serde(rename = "tokenID")]
    pub token_id: String,
    pub price: String,
    pub size: String,
    pub side: String,
}

#[allow(dead_code)]
pub async fn get_open_orders(
    client: &reqwest::Client,
    api_key: &str,
    secret: &str,
    passphrase: &str,
    address: &str,
) -> Result<Vec<OpenOrder>> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .to_string();

    let path = "/orders";
    let l2_sig = sign_l2_request(secret, &timestamp, "GET", path, "");

    let url = format!("{}{}", CLOB_BASE, path);
    let resp = client
        .get(&url)
        .header("POLY_ADDRESS", address)
        .header("POLY_SIGNATURE", &l2_sig)
        .header("POLY_TIMESTAMP", &timestamp)
        .header("POLY_NONCE", "0")
        .header("POLY_API_KEY", api_key)
        .header("POLY_PASSPHRASE", passphrase)
        .send()
        .await?;

    if !resp.status().is_success() {
        bail!(
            "Failed to fetch open orders: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }

    let orders: Vec<OpenOrder> = resp.json().await?;
    Ok(orders)
}

// --- Cancel order ---

#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CancelRequest {
    order_id: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CancelResponse {
    pub success: Option<bool>,
    pub error: Option<String>,
}

#[allow(dead_code)]
pub async fn cancel_order(
    client: &reqwest::Client,
    api_key: &str,
    secret: &str,
    passphrase: &str,
    address: &str,
    order_id: &str,
) -> Result<CancelResponse> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .to_string();

    let request = CancelRequest {
        order_id: order_id.to_string(),
    };
    let body = serde_json::to_string(&request)?;
    let path = "/order";
    let l2_sig = sign_l2_request(secret, &timestamp, "DELETE", path, &body);

    let url = format!("{}{}", CLOB_BASE, path);
    let resp = client
        .delete(&url)
        .header("POLY_ADDRESS", address)
        .header("POLY_SIGNATURE", &l2_sig)
        .header("POLY_TIMESTAMP", &timestamp)
        .header("POLY_NONCE", "0")
        .header("POLY_API_KEY", api_key)
        .header("POLY_PASSPHRASE", passphrase)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        bail!(
            "Order cancellation failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }

    let cancel_resp: CancelResponse = resp.json().await?;
    Ok(cancel_resp)
}
