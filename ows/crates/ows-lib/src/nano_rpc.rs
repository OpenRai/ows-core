//! Nano RPC helpers (account_info, work_generate, process).
//!
//! Uses `curl` for HTTP, consistent with the rest of ows-lib (no added HTTP deps).
//!
//! PoW is local-first by default (via `nano-rspow`), falling back to remote RPC.
//! Control via `NANO_WORK_MODE` env: `"auto"` (default), `"local"`, `"remote"`.

use crate::error::OwsLibError;
use std::process::Command;

/// Call a Nano RPC action via curl and return the parsed JSON response.
fn nano_rpc_call(
    rpc_url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, OwsLibError> {
    let body_str = body.to_string();
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body_str,
            rpc_url,
        ])
        .output()
        .map_err(|e| OwsLibError::BroadcastFailed(format!("failed to run curl: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OwsLibError::BroadcastFailed(format!(
            "Nano RPC call failed: {stderr}"
        )));
    }

    let resp_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&resp_str)?;

    // Check for Nano RPC error field
    if let Some(error) = parsed.get("error") {
        let msg = error.as_str().unwrap_or("unknown error");
        return Err(OwsLibError::BroadcastFailed(format!(
            "Nano RPC error: {msg}"
        )));
    }

    Ok(parsed)
}

/// Account info from the Nano network.
#[derive(Debug, Clone)]
pub struct NanoAccountInfo {
    /// Current frontier (head block hash), hex-encoded.
    pub frontier: String,
    /// Current balance in raw (decimal string).
    pub balance: String,
    /// Representative nano_ address.
    pub representative: String,
}

/// Query `account_info` for a Nano account.
///
/// Returns `None` if the account is not yet opened (no blocks published).
pub fn account_info(rpc_url: &str, account: &str) -> Result<Option<NanoAccountInfo>, OwsLibError> {
    let body = serde_json::json!({
        "action": "account_info",
        "account": account,
        "representative": "true"
    });

    match nano_rpc_call(rpc_url, &body) {
        Ok(resp) => {
            let frontier = resp["frontier"]
                .as_str()
                .ok_or_else(|| {
                    OwsLibError::BroadcastFailed("no frontier in account_info response".into())
                })?
                .to_string();
            let balance = resp["balance"]
                .as_str()
                .ok_or_else(|| {
                    OwsLibError::BroadcastFailed("no balance in account_info response".into())
                })?
                .to_string();
            let representative = resp["representative"]
                .as_str()
                .ok_or_else(|| {
                    OwsLibError::BroadcastFailed(
                        "no representative in account_info response".into(),
                    )
                })?
                .to_string();

            Ok(Some(NanoAccountInfo {
                frontier,
                balance,
                representative,
            }))
        }
        Err(OwsLibError::BroadcastFailed(msg)) if msg.contains("Account not found") => Ok(None),
        Err(e) => Err(e),
    }
}

/// Request proof-of-work from a single RPC endpoint.
fn work_generate_single(
    rpc_url: &str,
    hash: &str,
    difficulty_hex: &str,
) -> Result<String, OwsLibError> {
    let body = serde_json::json!({
        "action": "work_generate",
        "hash": hash,
        "difficulty": difficulty_hex
    });

    let resp = nano_rpc_call(rpc_url, &body)?;

    resp["work"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| OwsLibError::BroadcastFailed("no work in work_generate response".into()))
}

/// PoW mode controlled by `NANO_WORK_MODE` env var.
enum PowMode {
    /// Local only — fail if local PoW can't produce a result.
    Local,
    /// Remote only — skip local, go straight to RPC.
    Remote,
    /// Local first, remote fallback on failure.
    Auto,
}

fn pow_mode() -> PowMode {
    match std::env::var("NANO_WORK_MODE").as_deref() {
        Ok("local") => PowMode::Local,
        Ok("remote") => PowMode::Remote,
        _ => PowMode::Auto,
    }
}

/// Try local PoW via `nano-rspow`.
#[cfg(feature = "local-pow")]
fn work_generate_local(hash_hex: &str, threshold: u64) -> Result<String, OwsLibError> {
    let hash_bytes = hex::decode(hash_hex).map_err(|e| {
        OwsLibError::BroadcastFailed(format!("invalid hash hex for local PoW: {e}"))
    })?;
    if hash_bytes.len() != 32 {
        return Err(OwsLibError::BroadcastFailed(format!(
            "hash must be 32 bytes for PoW, got {}",
            hash_bytes.len()
        )));
    }
    let hash: [u8; 32] = hash_bytes.try_into().unwrap();

    match nano_rspow::work_generate(&hash, threshold) {
        Some(result) => Ok(result.nonce_hex()),
        None => Err(OwsLibError::BroadcastFailed(
            "local PoW generation failed".into(),
        )),
    }
}

#[cfg(not(feature = "local-pow"))]
fn work_generate_local(_hash_hex: &str, _threshold: u64) -> Result<String, OwsLibError> {
    Err(OwsLibError::BroadcastFailed(
        "local PoW not available (enable `local-pow` feature)".into(),
    ))
}

/// Request proof-of-work with local-first, remote-fallback strategy.
///
/// Controlled by `NANO_WORK_MODE` env var:
/// - `"auto"` (default): local first, remote fallback on failure
/// - `"local"`: local PoW only, fail if it can't produce a result
/// - `"remote"`: remote RPC only, skip local
///
/// Remote endpoints are tried in order:
/// 1. The primary `rpc_url`
/// 2. URLs from `NANO_WORK_URL` env var (semicolon-separated)
pub fn work_generate(rpc_url: &str, hash: &str, threshold: u64) -> Result<String, OwsLibError> {
    match pow_mode() {
        PowMode::Remote => {
            return work_generate_remote(rpc_url, hash, threshold);
        }
        PowMode::Local => {
            return work_generate_local(hash, threshold);
        }
        PowMode::Auto => {}
    }

    // Auto mode: try local first, fall back to remote
    if let Ok(work) = work_generate_local(hash, threshold) {
        return Ok(work);
    }

    eprintln!("  Local PoW failed, falling back to remote");
    work_generate_remote(rpc_url, hash, threshold)
}

/// Remote PoW fallback — tries the primary RPC then `NANO_WORK_URL` endpoints.
fn work_generate_remote(
    rpc_url: &str,
    hash: &str,
    threshold: u64,
) -> Result<String, OwsLibError> {
    let difficulty_hex = format!("{:016x}", threshold);

    let mut endpoints: Vec<String> = vec![rpc_url.to_string()];

    if let Ok(urls) = std::env::var("NANO_WORK_URL") {
        for url in urls.split(';') {
            let url = url.trim();
            if !url.is_empty() && url != rpc_url {
                endpoints.push(url.to_string());
            }
        }
    }

    let mut last_error = None;

    for endpoint in &endpoints {
        match work_generate_single(endpoint, hash, &difficulty_hex) {
            Ok(work) => return Ok(work),
            Err(e) => {
                eprintln!("  PoW failed on {endpoint}: {e}");
                last_error = Some(e);
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| OwsLibError::BroadcastFailed("no PoW endpoints available".into())))
}

/// Publish a block to the Nano network via `process` RPC.
///
/// Returns the block hash on success.
pub fn process_block(
    rpc_url: &str,
    block_json: &serde_json::Value,
    subtype: &str,
) -> Result<String, OwsLibError> {
    let body = serde_json::json!({
        "action": "process",
        "json_block": "true",
        "subtype": subtype,
        "block": block_json
    });

    let resp = nano_rpc_call(rpc_url, &body)?;

    resp["hash"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| OwsLibError::BroadcastFailed(format!("no hash in process response: {resp}")))
}

/// Epoch 2 send/change threshold.
///
/// When the `local-pow` feature is enabled, sourced directly from
/// `nano_rspow::thresholds` to avoid any duplication. When disabled,
/// falls back to the same hard-coded value.
#[cfg(feature = "local-pow")]
pub use nano_rspow::thresholds::EPOCH2_SEND;

#[cfg(not(feature = "local-pow"))]
pub const EPOCH2_SEND: u64 = 0xffff_fff8_0000_0000;

/// Epoch 2 receive threshold.
#[cfg(feature = "local-pow")]
pub use nano_rspow::thresholds::EPOCH2_RECEIVE;

#[cfg(not(feature = "local-pow"))]
pub const EPOCH2_RECEIVE: u64 = 0xffff_fe00_0000_0000;
