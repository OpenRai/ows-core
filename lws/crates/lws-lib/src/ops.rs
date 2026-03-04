use std::path::Path;
use std::process::Command;

use lws_core::{ChainType, Config, EncryptedWallet, KeyType, WalletAccount};
use lws_signer::{
    decrypt, encrypt, signer_for_chain, CryptoEnvelope, HdDeriver, Mnemonic, MnemonicStrength,
};

use crate::error::LwsLibError;
use crate::types::{SendResult, SignResult, WalletInfo};
use crate::vault;

/// Returns a default CAIP-2 chain reference for a given chain type.
fn default_chain_reference(chain: ChainType) -> &'static str {
    match chain {
        ChainType::Evm => "1",
        ChainType::Solana => "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
        ChainType::Bitcoin => "000000000019d6689c085ae165831e93",
        ChainType::Cosmos => "cosmoshub-4",
        ChainType::Tron => "mainnet",
    }
}

/// Convert an EncryptedWallet to the binding-friendly WalletInfo.
fn wallet_to_info(w: &EncryptedWallet) -> WalletInfo {
    let (address, derivation_path) = w
        .accounts
        .first()
        .map(|a| (a.address.clone(), a.derivation_path.clone()))
        .unwrap_or_default();

    WalletInfo {
        id: w.id.clone(),
        name: w.name.clone(),
        chain: w.chain_type,
        address,
        derivation_path,
        created_at: w.created_at.clone(),
    }
}

fn parse_chain(s: &str) -> Result<ChainType, LwsLibError> {
    s.parse::<ChainType>()
        .map_err(|e| LwsLibError::InvalidInput(e))
}

fn validate_passphrase(passphrase: &str) -> Result<(), LwsLibError> {
    if passphrase.len() < 12 {
        return Err(LwsLibError::InvalidInput(
            "passphrase must be at least 12 characters".into(),
        ));
    }
    Ok(())
}

/// Generate a new BIP-39 mnemonic phrase.
pub fn generate_mnemonic(words: u32) -> Result<String, LwsLibError> {
    let strength = match words {
        12 => MnemonicStrength::Words12,
        24 => MnemonicStrength::Words24,
        _ => return Err(LwsLibError::InvalidInput("words must be 12 or 24".into())),
    };

    let mnemonic = Mnemonic::generate(strength)?;
    let phrase = mnemonic.phrase();
    String::from_utf8(phrase.expose().to_vec())
        .map_err(|e| LwsLibError::InvalidInput(format!("invalid UTF-8 in mnemonic: {e}")))
}

/// Derive an address from a mnemonic phrase for the given chain.
pub fn derive_address(
    mnemonic_phrase: &str,
    chain: &str,
    index: Option<u32>,
) -> Result<String, LwsLibError> {
    let chain = parse_chain(chain)?;
    let mnemonic = Mnemonic::from_phrase(mnemonic_phrase)?;
    let signer = signer_for_chain(chain);
    let path = signer.default_derivation_path(index.unwrap_or(0));
    let curve = signer.curve();

    let key = HdDeriver::derive_from_mnemonic(&mnemonic, "", &path, curve)?;
    let address = signer.derive_address(key.expose())?;
    Ok(address)
}

/// Create a new wallet: generates mnemonic, derives address, encrypts, saves to vault.
pub fn create_wallet(
    name: &str,
    chain: &str,
    words: Option<u32>,
    passphrase: &str,
    vault_path: Option<&Path>,
) -> Result<WalletInfo, LwsLibError> {
    validate_passphrase(passphrase)?;
    let chain = parse_chain(chain)?;
    let words = words.unwrap_or(12);
    let strength = match words {
        12 => MnemonicStrength::Words12,
        24 => MnemonicStrength::Words24,
        _ => return Err(LwsLibError::InvalidInput("words must be 12 or 24".into())),
    };

    if vault::wallet_name_exists(name, vault_path)? {
        return Err(LwsLibError::WalletNameExists(name.to_string()));
    }

    let mnemonic = Mnemonic::generate(strength)?;
    let signer = signer_for_chain(chain);
    let path = signer.default_derivation_path(0);
    let curve = signer.curve();

    let key = HdDeriver::derive_from_mnemonic(&mnemonic, "", &path, curve)?;
    let address = signer.derive_address(key.expose())?;

    let chain_id_str = format!("{}:{}", chain.namespace(), default_chain_reference(chain));
    let account_id_str = format!("{chain_id_str}:{address}");

    let phrase = mnemonic.phrase();
    let crypto_envelope = encrypt(phrase.expose(), passphrase)?;
    let crypto_json = serde_json::to_value(&crypto_envelope)?;

    let wallet_id = uuid::Uuid::new_v4().to_string();

    let wallet = EncryptedWallet::new(
        wallet_id,
        name.to_string(),
        chain,
        vec![WalletAccount {
            account_id: account_id_str,
            address: address.clone(),
            chain_id: chain_id_str,
            derivation_path: path,
        }],
        crypto_json,
        KeyType::Mnemonic,
    );

    vault::save_encrypted_wallet(&wallet, vault_path)?;
    Ok(wallet_to_info(&wallet))
}

/// Import a wallet from a mnemonic phrase.
pub fn import_wallet_mnemonic(
    name: &str,
    chain: &str,
    mnemonic_phrase: &str,
    passphrase: &str,
    index: Option<u32>,
    vault_path: Option<&Path>,
) -> Result<WalletInfo, LwsLibError> {
    validate_passphrase(passphrase)?;
    let chain = parse_chain(chain)?;
    let index = index.unwrap_or(0);

    if vault::wallet_name_exists(name, vault_path)? {
        return Err(LwsLibError::WalletNameExists(name.to_string()));
    }

    let mnemonic = Mnemonic::from_phrase(mnemonic_phrase)?;
    let signer = signer_for_chain(chain);
    let path = signer.default_derivation_path(index);
    let curve = signer.curve();

    let key = HdDeriver::derive_from_mnemonic(&mnemonic, "", &path, curve)?;
    let address = signer.derive_address(key.expose())?;

    let phrase = mnemonic.phrase();
    let crypto_envelope = encrypt(phrase.expose(), passphrase)?;
    let crypto_json = serde_json::to_value(&crypto_envelope)?;

    let wallet_id = uuid::Uuid::new_v4().to_string();
    let chain_id_str = format!("{}:{}", chain.namespace(), default_chain_reference(chain));
    let account_id_str = format!("{chain_id_str}:{address}");

    let wallet = EncryptedWallet::new(
        wallet_id,
        name.to_string(),
        chain,
        vec![WalletAccount {
            account_id: account_id_str,
            address: address.clone(),
            chain_id: chain_id_str,
            derivation_path: path,
        }],
        crypto_json,
        KeyType::Mnemonic,
    );

    vault::save_encrypted_wallet(&wallet, vault_path)?;
    Ok(wallet_to_info(&wallet))
}

/// Import a wallet from a hex-encoded private key.
pub fn import_wallet_private_key(
    name: &str,
    chain: &str,
    private_key_hex: &str,
    passphrase: &str,
    vault_path: Option<&Path>,
) -> Result<WalletInfo, LwsLibError> {
    validate_passphrase(passphrase)?;
    let chain = parse_chain(chain)?;

    if vault::wallet_name_exists(name, vault_path)? {
        return Err(LwsLibError::WalletNameExists(name.to_string()));
    }

    let hex_trimmed = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);
    let key_bytes = hex::decode(hex_trimmed)
        .map_err(|e| LwsLibError::InvalidInput(format!("invalid hex private key: {e}")))?;

    let signer = signer_for_chain(chain);
    let address = signer.derive_address(&key_bytes)?;

    let crypto_envelope = encrypt(&key_bytes, passphrase)?;
    let crypto_json = serde_json::to_value(&crypto_envelope)?;

    let wallet_id = uuid::Uuid::new_v4().to_string();
    let chain_id_str = format!("{}:{}", chain.namespace(), default_chain_reference(chain));
    let account_id_str = format!("{chain_id_str}:{address}");

    let wallet = EncryptedWallet::new(
        wallet_id,
        name.to_string(),
        chain,
        vec![WalletAccount {
            account_id: account_id_str,
            address: address.clone(),
            chain_id: chain_id_str,
            derivation_path: String::new(),
        }],
        crypto_json,
        KeyType::PrivateKey,
    );

    vault::save_encrypted_wallet(&wallet, vault_path)?;
    Ok(wallet_to_info(&wallet))
}

/// List all wallets in the vault.
pub fn list_wallets(vault_path: Option<&Path>) -> Result<Vec<WalletInfo>, LwsLibError> {
    let wallets = vault::list_encrypted_wallets(vault_path)?;
    Ok(wallets.iter().map(wallet_to_info).collect())
}

/// Get a single wallet by name or ID.
pub fn get_wallet(
    name_or_id: &str,
    vault_path: Option<&Path>,
) -> Result<WalletInfo, LwsLibError> {
    let wallet = vault::load_wallet_by_name_or_id(name_or_id, vault_path)?;
    Ok(wallet_to_info(&wallet))
}

/// Delete a wallet from the vault.
pub fn delete_wallet(
    name_or_id: &str,
    vault_path: Option<&Path>,
) -> Result<(), LwsLibError> {
    let wallet = vault::load_wallet_by_name_or_id(name_or_id, vault_path)?;
    vault::delete_wallet_file(&wallet.id, vault_path)?;
    Ok(())
}

/// Export a wallet's secret (mnemonic or private key).
pub fn export_wallet(
    name_or_id: &str,
    passphrase: &str,
    vault_path: Option<&Path>,
) -> Result<String, LwsLibError> {
    let wallet = vault::load_wallet_by_name_or_id(name_or_id, vault_path)?;
    let envelope: CryptoEnvelope = serde_json::from_value(wallet.crypto.clone())?;
    let secret = decrypt(&envelope, passphrase)?;

    String::from_utf8(secret.expose().to_vec())
        .map_err(|_| LwsLibError::InvalidInput("wallet contains invalid UTF-8 secret".into()))
}

/// Rename a wallet.
pub fn rename_wallet(
    name_or_id: &str,
    new_name: &str,
    vault_path: Option<&Path>,
) -> Result<(), LwsLibError> {
    let mut wallet = vault::load_wallet_by_name_or_id(name_or_id, vault_path)?;

    if wallet.name == new_name {
        return Ok(());
    }

    if vault::wallet_name_exists(new_name, vault_path)? {
        return Err(LwsLibError::WalletNameExists(new_name.to_string()));
    }

    wallet.name = new_name.to_string();
    vault::save_encrypted_wallet(&wallet, vault_path)?;
    Ok(())
}

/// Sign a transaction. Returns hex-encoded signature.
///
/// The `wallet` parameter is a wallet name or ID.
/// The `tx_hex` parameter is the hex-encoded unsigned transaction bytes.
pub fn sign_transaction(
    wallet: &str,
    chain: &str,
    tx_hex: &str,
    passphrase: &str,
    index: Option<u32>,
    vault_path: Option<&Path>,
) -> Result<SignResult, LwsLibError> {
    let chain = parse_chain(chain)?;
    let mnemonic_str = decrypt_wallet_secret(wallet, passphrase, vault_path)?;
    let mnemonic = Mnemonic::from_phrase(&mnemonic_str)?;

    let tx_hex_clean = tx_hex.strip_prefix("0x").unwrap_or(tx_hex);
    let tx_bytes = hex::decode(tx_hex_clean)
        .map_err(|e| LwsLibError::InvalidInput(format!("invalid hex transaction: {e}")))?;

    let signer = signer_for_chain(chain);
    let path = signer.default_derivation_path(index.unwrap_or(0));
    let curve = signer.curve();

    let key = HdDeriver::derive_from_mnemonic(&mnemonic, "", &path, curve)?;
    let output = signer.sign_transaction(key.expose(), &tx_bytes)?;

    Ok(SignResult {
        signature: hex::encode(&output.signature),
        recovery_id: output.recovery_id,
    })
}

/// Sign a message. Returns hex-encoded signature.
///
/// The `wallet` parameter is a wallet name or ID.
/// The `encoding` parameter is "utf8" or "hex" (defaults to "utf8").
pub fn sign_message(
    wallet: &str,
    chain: &str,
    message: &str,
    passphrase: &str,
    encoding: Option<&str>,
    index: Option<u32>,
    vault_path: Option<&Path>,
) -> Result<SignResult, LwsLibError> {
    let chain = parse_chain(chain)?;
    let mnemonic_str = decrypt_wallet_secret(wallet, passphrase, vault_path)?;
    let mnemonic = Mnemonic::from_phrase(&mnemonic_str)?;

    let encoding = encoding.unwrap_or("utf8");
    let msg_bytes = match encoding {
        "utf8" => message.as_bytes().to_vec(),
        "hex" => hex::decode(message)
            .map_err(|e| LwsLibError::InvalidInput(format!("invalid hex message: {e}")))?,
        _ => {
            return Err(LwsLibError::InvalidInput(format!(
                "unsupported encoding: {encoding} (use 'utf8' or 'hex')"
            )))
        }
    };

    let signer = signer_for_chain(chain);
    let path = signer.default_derivation_path(index.unwrap_or(0));
    let curve = signer.curve();

    let key = HdDeriver::derive_from_mnemonic(&mnemonic, "", &path, curve)?;
    let output = signer.sign_message(key.expose(), &msg_bytes)?;

    Ok(SignResult {
        signature: hex::encode(&output.signature),
        recovery_id: output.recovery_id,
    })
}

/// Sign and broadcast a transaction. Returns the transaction hash.
pub fn sign_and_send(
    wallet: &str,
    chain: &str,
    tx_hex: &str,
    passphrase: &str,
    index: Option<u32>,
    rpc_url: Option<&str>,
    vault_path: Option<&Path>,
) -> Result<SendResult, LwsLibError> {
    let chain_type = parse_chain(chain)?;

    // 1. Sign
    let mnemonic_str = decrypt_wallet_secret(wallet, passphrase, vault_path)?;
    let mnemonic = Mnemonic::from_phrase(&mnemonic_str)?;

    let tx_hex_clean = tx_hex.strip_prefix("0x").unwrap_or(tx_hex);
    let tx_bytes = hex::decode(tx_hex_clean)
        .map_err(|e| LwsLibError::InvalidInput(format!("invalid hex transaction: {e}")))?;

    let signer = signer_for_chain(chain_type);
    let path = signer.default_derivation_path(index.unwrap_or(0));
    let curve = signer.curve();

    let key = HdDeriver::derive_from_mnemonic(&mnemonic, "", &path, curve)?;
    let output = signer.sign_transaction(key.expose(), &tx_bytes)?;

    // 2. Resolve RPC URL
    let rpc = resolve_rpc_url(chain_type, rpc_url)?;

    // 3. Broadcast
    let tx_hash = broadcast(chain_type, &rpc, &output.signature)?;

    Ok(SendResult { tx_hash })
}

// --- internal helpers ---

/// Decrypt a wallet's secret material and return it as a string.
fn decrypt_wallet_secret(
    wallet_name_or_id: &str,
    passphrase: &str,
    vault_path: Option<&Path>,
) -> Result<String, LwsLibError> {
    let wallet = vault::load_wallet_by_name_or_id(wallet_name_or_id, vault_path)?;
    let envelope: CryptoEnvelope = serde_json::from_value(wallet.crypto.clone())?;
    let secret = decrypt(&envelope, passphrase)?;

    String::from_utf8(secret.expose().to_vec())
        .map_err(|_| LwsLibError::InvalidInput("wallet contains invalid UTF-8 secret".into()))
}

/// Resolve the RPC URL: explicit > config override > built-in default.
fn resolve_rpc_url(chain: ChainType, explicit: Option<&str>) -> Result<String, LwsLibError> {
    if let Some(url) = explicit {
        return Ok(url.to_string());
    }

    let config = Config::load_or_default();
    let defaults = Config::default_rpc();
    let namespace = chain.namespace();

    for (key, url) in &config.rpc {
        if key.starts_with(namespace) {
            return Ok(url.clone());
        }
    }

    for (key, url) in &defaults {
        if key.starts_with(namespace) {
            return Ok(url.clone());
        }
    }

    Err(LwsLibError::InvalidInput(format!(
        "no RPC URL configured for chain '{chain}'"
    )))
}

/// Broadcast a signed transaction via curl, dispatching per chain type.
fn broadcast(chain: ChainType, rpc_url: &str, signed_bytes: &[u8]) -> Result<String, LwsLibError> {
    match chain {
        ChainType::Evm => broadcast_evm(rpc_url, signed_bytes),
        ChainType::Solana => broadcast_solana(rpc_url, signed_bytes),
        ChainType::Bitcoin => broadcast_bitcoin(rpc_url, signed_bytes),
        ChainType::Cosmos => broadcast_cosmos(rpc_url, signed_bytes),
        ChainType::Tron => broadcast_tron(rpc_url, signed_bytes),
    }
}

fn broadcast_evm(rpc_url: &str, signed_bytes: &[u8]) -> Result<String, LwsLibError> {
    let hex_tx = format!("0x{}", hex::encode(signed_bytes));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_sendRawTransaction",
        "params": [hex_tx],
        "id": 1
    });
    let resp = curl_post_json(rpc_url, &body.to_string())?;
    extract_json_field(&resp, "result")
}

fn broadcast_solana(rpc_url: &str, signed_bytes: &[u8]) -> Result<String, LwsLibError> {
    use base64::Engine;
    let b64_tx = base64::engine::general_purpose::STANDARD.encode(signed_bytes);
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "sendTransaction",
        "params": [b64_tx],
        "id": 1
    });
    let resp = curl_post_json(rpc_url, &body.to_string())?;
    extract_json_field(&resp, "result")
}

fn broadcast_bitcoin(rpc_url: &str, signed_bytes: &[u8]) -> Result<String, LwsLibError> {
    let hex_tx = hex::encode(signed_bytes);
    let url = format!("{}/tx", rpc_url.trim_end_matches('/'));
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "-X", "POST",
            "-H", "Content-Type: text/plain",
            "-d", &hex_tx,
            &url,
        ])
        .output()
        .map_err(|e| LwsLibError::BroadcastFailed(format!("failed to run curl: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LwsLibError::BroadcastFailed(format!("broadcast failed: {stderr}")));
    }

    let tx_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tx_hash.is_empty() {
        return Err(LwsLibError::BroadcastFailed("empty response from broadcast".into()));
    }
    Ok(tx_hash)
}

fn broadcast_cosmos(rpc_url: &str, signed_bytes: &[u8]) -> Result<String, LwsLibError> {
    use base64::Engine;
    let b64_tx = base64::engine::general_purpose::STANDARD.encode(signed_bytes);
    let url = format!(
        "{}/cosmos/tx/v1beta1/txs",
        rpc_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "tx_bytes": b64_tx,
        "mode": "BROADCAST_MODE_SYNC"
    });
    let resp = curl_post_json(&url, &body.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&resp)?;
    parsed["tx_response"]["txhash"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| LwsLibError::BroadcastFailed(format!("no txhash in response: {resp}")))
}

fn broadcast_tron(rpc_url: &str, signed_bytes: &[u8]) -> Result<String, LwsLibError> {
    let hex_tx = hex::encode(signed_bytes);
    let url = format!(
        "{}/wallet/broadcasthex",
        rpc_url.trim_end_matches('/')
    );
    let body = serde_json::json!({ "transaction": hex_tx });
    let resp = curl_post_json(&url, &body.to_string())?;
    extract_json_field(&resp, "txid")
}

fn curl_post_json(url: &str, body: &str) -> Result<String, LwsLibError> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", body,
            url,
        ])
        .output()
        .map_err(|e| LwsLibError::BroadcastFailed(format!("failed to run curl: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LwsLibError::BroadcastFailed(format!("broadcast failed: {stderr}")));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn extract_json_field(json_str: &str, field: &str) -> Result<String, LwsLibError> {
    let parsed: serde_json::Value = serde_json::from_str(json_str)?;

    if let Some(error) = parsed.get("error") {
        return Err(LwsLibError::BroadcastFailed(format!("RPC error: {error}")));
    }

    parsed[field]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| LwsLibError::BroadcastFailed(format!("no '{field}' in response: {json_str}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic_12() {
        let phrase = generate_mnemonic(12).unwrap();
        assert_eq!(phrase.split_whitespace().count(), 12);
    }

    #[test]
    fn test_generate_mnemonic_24() {
        let phrase = generate_mnemonic(24).unwrap();
        assert_eq!(phrase.split_whitespace().count(), 24);
    }

    #[test]
    fn test_generate_mnemonic_invalid() {
        assert!(generate_mnemonic(15).is_err());
    }

    #[test]
    fn test_derive_address_evm() {
        let phrase = generate_mnemonic(12).unwrap();
        let addr = derive_address(&phrase, "evm", None).unwrap();
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 42);
    }

    #[test]
    fn test_derive_address_solana() {
        let phrase = generate_mnemonic(12).unwrap();
        let addr = derive_address(&phrase, "solana", None).unwrap();
        assert!(!addr.is_empty());
    }

    #[test]
    fn test_create_and_list_wallets() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        let info = create_wallet("test-wallet", "evm", None, "supersecretpass!", Some(vault))
            .unwrap();

        assert_eq!(info.name, "test-wallet");
        assert_eq!(info.chain, ChainType::Evm);
        assert!(info.address.starts_with("0x"));

        let wallets = list_wallets(Some(vault)).unwrap();
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].id, info.id);
    }

    #[test]
    fn test_create_wallet_duplicate_name() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        create_wallet("dup-name", "evm", None, "supersecretpass!", Some(vault)).unwrap();
        let err = create_wallet("dup-name", "evm", None, "supersecretpass!", Some(vault));
        assert!(err.is_err());
    }

    #[test]
    fn test_get_wallet() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        let info = create_wallet("lookup-test", "evm", None, "supersecretpass!", Some(vault))
            .unwrap();

        // By name
        let found = get_wallet("lookup-test", Some(vault)).unwrap();
        assert_eq!(found.id, info.id);

        // By ID
        let found = get_wallet(&info.id, Some(vault)).unwrap();
        assert_eq!(found.name, "lookup-test");
    }

    #[test]
    fn test_delete_wallet() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        let info = create_wallet("del-test", "evm", None, "supersecretpass!", Some(vault))
            .unwrap();

        delete_wallet(&info.id, Some(vault)).unwrap();
        assert!(list_wallets(Some(vault)).unwrap().is_empty());
    }

    #[test]
    fn test_export_wallet() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        create_wallet("export-test", "evm", None, "supersecretpass!", Some(vault)).unwrap();

        let secret = export_wallet("export-test", "supersecretpass!", Some(vault)).unwrap();
        // The secret should be a valid mnemonic (12 words)
        assert_eq!(secret.split_whitespace().count(), 12);
    }

    #[test]
    fn test_rename_wallet() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        create_wallet("old-name", "evm", None, "supersecretpass!", Some(vault)).unwrap();
        rename_wallet("old-name", "new-name", Some(vault)).unwrap();

        let found = get_wallet("new-name", Some(vault)).unwrap();
        assert_eq!(found.name, "new-name");
        assert!(get_wallet("old-name", Some(vault)).is_err());
    }

    #[test]
    fn test_import_wallet_mnemonic() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        let phrase = generate_mnemonic(12).unwrap();
        let expected_addr = derive_address(&phrase, "evm", None).unwrap();

        let info = import_wallet_mnemonic(
            "imported",
            "evm",
            &phrase,
            "supersecretpass!",
            None,
            Some(vault),
        )
        .unwrap();

        assert_eq!(info.name, "imported");
        assert_eq!(info.address, expected_addr);
    }

    #[test]
    fn test_sign_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        create_wallet("signer", "evm", None, "supersecretpass!", Some(vault)).unwrap();

        // Sign a dummy 32-byte "transaction" (for EVM, sign_transaction hashes the input)
        let tx_hex = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let result =
            sign_transaction("signer", "evm", tx_hex, "supersecretpass!", None, Some(vault))
                .unwrap();

        assert!(!result.signature.is_empty());
        assert!(result.recovery_id.is_some());
    }

    #[test]
    fn test_sign_message() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        create_wallet("msg-signer", "evm", None, "supersecretpass!", Some(vault)).unwrap();

        let result = sign_message(
            "msg-signer",
            "evm",
            "hello world",
            "supersecretpass!",
            None,
            None,
            Some(vault),
        )
        .unwrap();

        assert!(!result.signature.is_empty());
    }

    #[test]
    fn test_passphrase_too_short() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();

        let err = create_wallet("short-pass", "evm", None, "short", Some(vault));
        assert!(err.is_err());
    }

}
