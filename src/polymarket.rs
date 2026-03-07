use std::str::FromStr;

use anyhow::{Context, Result};
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::{LocalSigner, Normal, Signer as _};
use polymarket_client_sdk::clob::types::SignatureType;
use polymarket_client_sdk::{bridge, clob, data, gamma, POLYGON};

use crate::config;

fn parse_signature_type(s: &str) -> SignatureType {
    match s {
        "proxy" => SignatureType::Proxy,
        "gnosis-safe" => SignatureType::GnosisSafe,
        _ => SignatureType::Eoa,
    }
}

/// Unauthenticated Gamma client for browsing markets
pub fn create_gamma_client() -> gamma::Client {
    gamma::Client::default()
}

/// Unauthenticated Bridge client for deposit addresses
pub fn create_bridge_client() -> bridge::Client {
    bridge::Client::default()
}

/// Unauthenticated Data client for positions
pub fn create_data_client() -> data::Client {
    data::Client::default()
}

/// Authenticated CLOB client for trading
pub async fn create_clob_client() -> Result<clob::Client<Authenticated<Normal>>> {
    let (key, sig_type) = config::polymarket_credentials()?;
    let signer = LocalSigner::from_str(&key)
        .context("Invalid Polymarket private key")?
        .with_chain_id(Some(POLYGON));
    let sig = parse_signature_type(&sig_type);

    clob::Client::default()
        .authentication_builder(&signer)
        .signature_type(sig)
        .authenticate()
        .await
        .context("Failed to authenticate with Polymarket CLOB")
}

/// Get the Polymarket wallet address from config
pub fn get_polymarket_address() -> Result<String> {
    let (key, _) = config::polymarket_credentials()?;
    let key_clean = key.strip_prefix("0x").unwrap_or(&key);
    address_from_private_key(key_clean)
}

fn address_from_private_key(hex_key: &str) -> Result<String> {
    let bytes = hex::decode(hex_key).context("Invalid hex private key")?;
    use k256::ecdsa::SigningKey;
    use sha3::{Digest, Keccak256};
    let signing_key =
        SigningKey::from_bytes(bytes.as_slice().into()).context("Invalid private key")?;
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_encoded_point(false);
    let public_key_bytes = &public_key.as_bytes()[1..];
    let mut hasher = Keccak256::new();
    hasher.update(public_key_bytes);
    let hash: [u8; 32] = hasher.finalize().into();
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}
