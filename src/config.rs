use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// On-disk config file (~/.fintool/config.toml)
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub wallet: WalletConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub api_keys: ApiKeysConfig,
    #[serde(default)]
    pub polymarket: PolymarketConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PolymarketConfig {
    /// Signature type: proxy (default), eoa, or gnosis-safe
    pub signature_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WalletConfig {
    /// Hex private key (with or without 0x prefix). Takes priority over wallet_json.
    pub private_key: Option<String>,
    /// Path to encrypted wallet JSON (keystore) file
    pub wallet_json: Option<String>,
    /// Passcode to decrypt the wallet JSON file
    pub wallet_passcode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    /// Use Hyperliquid testnet (default: false)
    #[serde(default)]
    pub testnet: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ApiKeysConfig {
    /// CryptoPanic API token for crypto news
    pub cryptopanic_token: Option<String>,
    /// NewsAPI key for stock news (optional)
    pub newsapi_key: Option<String>,
    /// OpenAI API key for enriched quote analysis
    pub openai_api_key: Option<String>,
    /// OpenAI model for quote analysis (default: gpt-4.1-mini)
    pub openai_model: Option<String>,
    /// Binance API key for spot/futures/options trading
    pub binance_api_key: Option<String>,
    /// Binance API secret for signing requests
    pub binance_api_secret: Option<String>,
    /// Coinbase Advanced Trade API key
    pub coinbase_api_key: Option<String>,
    /// Coinbase Advanced Trade API secret
    pub coinbase_api_secret: Option<String>,
}

/// Resolved runtime config
pub struct HlConfig {
    pub private_key: String,
    pub address: String,
    pub testnet: bool,
}

impl HlConfig {}

/// Return the config file path (~/.fintool/config.toml)
pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".fintool")
        .join("config.toml")
}

/// Load the config file, returning defaults if it doesn't exist
pub fn load_config_file() -> Result<ConfigFile> {
    let path = config_path();
    if !path.exists() {
        return Ok(ConfigFile::default());
    }
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let cfg: ConfigFile = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(cfg)
}

/// Create a default config file with comments if it doesn't exist
pub fn init_config() -> Result<(PathBuf, bool)> {
    let path = config_path();
    if path.exists() {
        return Ok((path, false));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Try to read config.toml.default from the project directory, fall back to embedded template
    let template = include_str!("../config.toml.default");
    std::fs::write(&path, template)?;
    Ok((path, true))
}

/// Load wallet config from config file
pub fn load_hl_config() -> Result<HlConfig> {
    let cfg = load_config_file()?;
    let testnet = cfg.network.testnet;

    // 1. Check config file private_key
    if let Some(ref key) = cfg.wallet.private_key {
        let key = key.strip_prefix("0x").unwrap_or(key).to_string();
        let address = address_from_key(&key)?;
        return Ok(HlConfig {
            private_key: key,
            address,
            testnet,
        });
    }

    // 2. Check config file wallet_json + wallet_passcode
    if let Some(ref path) = cfg.wallet.wallet_json {
        let passcode = cfg.wallet.wallet_passcode.as_deref().unwrap_or("");
        return load_from_keystore(path, passcode, testnet);
    }

    let config_file = config_path();
    bail!(
        "No wallet configured.\n\
         \n\
         Configure in {}:\n\
         \n\
         [wallet]\n\
         private_key = \"0x...\"\n\
         \n\
         Run `fintool init` to create a default config file.",
        config_file.display()
    )
}

fn load_from_keystore(path: &str, passcode: &str, testnet: bool) -> Result<HlConfig> {
    let path = shellexpand::tilde(path).to_string();
    let full_path = std::path::Path::new(&path);

    let decrypted = eth_keystore::decrypt_key(full_path, passcode)
        .context("Failed to decrypt wallet JSON — wrong passcode?")?;

    let key = hex::encode(&decrypted);
    let address = address_from_key(&key)?;
    Ok(HlConfig {
        private_key: key,
        address,
        testnet,
    })
}

fn address_from_key(hex_key: &str) -> Result<String> {
    let bytes = hex::decode(hex_key).context("Invalid hex private key")?;
    if bytes.len() != 32 {
        bail!("Private key must be 32 bytes");
    }
    use k256::ecdsa::SigningKey;
    let signing_key =
        SigningKey::from_bytes(bytes.as_slice().into()).context("Invalid private key")?;
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_encoded_point(false);
    let public_key_bytes = &public_key.as_bytes()[1..]; // skip 0x04 prefix
    let hash = keccak256(public_key_bytes);
    let address = format!("0x{}", hex::encode(&hash[12..]));
    Ok(address)
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Info-only API URL (no signing needed) — reads testnet from config
pub fn info_url() -> String {
    let cfg = load_config_file().unwrap_or_default();
    let testnet = cfg.network.testnet;
    if testnet {
        "https://api.hyperliquid-testnet.xyz/info".to_string()
    } else {
        "https://api.hyperliquid.xyz/info".to_string()
    }
}

/// Get the OpenAI API key
pub fn openai_api_key() -> Option<String> {
    load_config_file().ok()?.api_keys.openai_api_key
}

pub fn openai_model() -> String {
    load_config_file()
        .ok()
        .and_then(|c| c.api_keys.openai_model)
        .unwrap_or_else(|| "gpt-4.1-mini".to_string())
}

/// Get Binance API credentials (key, secret)
pub fn binance_credentials() -> Option<(String, String)> {
    let cfg = load_config_file().ok()?;
    Some((
        cfg.api_keys.binance_api_key?,
        cfg.api_keys.binance_api_secret?,
    ))
}

/// Get Polymarket credentials (private_key, signature_type)
/// Get wallet private key + Polymarket signature type for Polymarket operations.
/// Uses the same [wallet] private_key as all other exchanges.
pub fn polymarket_credentials() -> Result<(String, String)> {
    let cfg = load_config_file()?;
    let key = cfg.wallet.private_key.ok_or_else(|| {
        anyhow::anyhow!(
            "No wallet private key configured.\n\
             Set [wallet] private_key in {}",
            config_path().display()
        )
    })?;
    let sig_type = cfg
        .polymarket
        .signature_type
        .unwrap_or_else(|| "proxy".to_string());
    Ok((key, sig_type))
}

/// Get Coinbase Advanced Trade API credentials (key, secret)
pub fn coinbase_credentials() -> Option<(String, String)> {
    let cfg = load_config_file().ok()?;
    Some((
        cfg.api_keys.coinbase_api_key?,
        cfg.api_keys.coinbase_api_secret?,
    ))
}
