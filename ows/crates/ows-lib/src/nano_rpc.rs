//! Nano RPC helpers (account_info, work_generate, process).
//!
//! Uses `curl` for HTTP, consistent with the rest of ows-lib (no added HTTP deps).

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
    difficulty: &str,
) -> Result<String, OwsLibError> {
    let body = serde_json::json!({
        "action": "work_generate",
        "hash": hash,
        "difficulty": difficulty
    });

    let resp = nano_rpc_call(rpc_url, &body)?;

    resp["work"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| OwsLibError::BroadcastFailed("no work in work_generate response".into()))
}

/// Default PoW fallback endpoint, tried when the primary RPC fails work_generate.
const FALLBACK_WORK_PEER: &str = "https://rpc.nano.to";

/// Request proof-of-work with multi-endpoint fallback.
///
/// Tries endpoints in order:
/// 1. The primary `rpc_url`
/// 2. Peers from `NANO_WORK_PEERS` env var (semicolon-separated URLs)
/// 3. Built-in fallback endpoint
///
/// All remote errors are collected and logged to stderr. If every remote fails
/// and `NANO_CPU_POW=1` is set, a future CPU fallback would go here.
pub fn work_generate(rpc_url: &str, hash: &str, difficulty: &str) -> Result<String, OwsLibError> {
    let mut endpoints: Vec<String> = vec![rpc_url.to_string()];

    if let Ok(peers) = std::env::var("NANO_WORK_PEERS") {
        for peer in peers.split(';') {
            let peer = peer.trim();
            if !peer.is_empty() && peer != rpc_url {
                endpoints.push(peer.to_string());
            }
        }
    }

    if !endpoints.iter().any(|e| e == FALLBACK_WORK_PEER) {
        endpoints.push(FALLBACK_WORK_PEER.to_string());
    }

    let mut last_error = None;

    for endpoint in &endpoints {
        match work_generate_single(endpoint, hash, difficulty) {
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

/// PoW difficulty thresholds (as hex strings for work_generate RPC).
pub const SEND_DIFFICULTY: &str = "fffffff800000000";
pub const RECEIVE_DIFFICULTY: &str = "fffffe0000000000";

// ─────────────────────────────────────────────────────────────────────────────
// Unit conversion (XNO ↔ raw)
// ─────────────────────────────────────────────────────────────────────────────

/// 1 XNO = 10^30 raw.
const RAW_PER_XNO: u128 = 1_000_000_000_000_000_000_000_000_000_000;

/// Convert a decimal XNO string to raw (u128).
///
/// Handles up to 30 decimal places (the full precision of raw).
/// Returns `None` for invalid or over-precise input.
pub fn xno_to_raw(xno: &str) -> Option<u128> {
    let xno = xno.trim();
    if xno.is_empty() {
        return None;
    }

    let parts: Vec<&str> = xno.split('.').collect();
    match parts.len() {
        1 => {
            let whole: u128 = parts[0].parse().ok()?;
            whole.checked_mul(RAW_PER_XNO)
        }
        2 => {
            let whole: u128 = if parts[0].is_empty() {
                0
            } else {
                parts[0].parse().ok()?
            };

            let frac_str = parts[1];
            if frac_str.len() > 30 {
                return None;
            }

            let padded = format!("{:0<30}", frac_str);
            let frac: u128 = padded.parse().ok()?;

            whole.checked_mul(RAW_PER_XNO)?.checked_add(frac)
        }
        _ => None,
    }
}

/// Convert raw (u128) to a decimal XNO string.
///
/// Always exact — includes enough decimal places with no trailing zeros.
pub fn raw_to_xno(raw: u128) -> String {
    let whole = raw / RAW_PER_XNO;
    let frac = raw % RAW_PER_XNO;

    if frac == 0 {
        return format!("{}.0", whole);
    }

    let frac_str = format!("{:030}", frac);
    let trimmed = frac_str.trim_end_matches('0');
    format!("{}.{}", whole, trimmed)
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional RPC calls for wallet actions
// ─────────────────────────────────────────────────────────────────────────────

/// A pending (receivable) block.
#[derive(Debug, Clone)]
pub struct PendingBlock {
    /// Send block hash hex.
    pub hash: String,
    /// Amount in raw.
    pub amount: u128,
    /// Source address (sender).
    pub source: String,
}

/// Query `receivable` (pending) blocks for a Nano account.
///
/// Returns pending send blocks sorted by amount (largest first).
/// `min_raw` filters out sends below a minimum amount (node-side threshold).
pub fn receivable(
    rpc_url: &str,
    account: &str,
    count: u32,
    min_raw: Option<u128>,
) -> Result<Vec<PendingBlock>, OwsLibError> {
    let mut req = serde_json::json!({
        "action": "receivable",
        "account": account,
        "count": count.to_string(),
        "source": "true",
        "sorting": "true",
    });

    if let Some(min) = min_raw {
        req["threshold"] = serde_json::Value::String(min.to_string());
    }

    let resp = nano_rpc_call(rpc_url, &req)?;

    let blocks = match resp.get("blocks") {
        Some(serde_json::Value::Object(map)) => map.clone(),
        // Empty string or missing means no pending blocks
        _ => return Ok(Vec::new()),
    };

    let mut result = Vec::new();
    for (hash_hex, info) in &blocks {
        let amount_str = info.get("amount").and_then(|v| v.as_str()).unwrap_or("0");
        let amount: u128 = amount_str.parse().unwrap_or(0);

        let source = info
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        result.push(PendingBlock {
            hash: hash_hex.clone(),
            amount,
            source,
        });
    }

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xno_to_raw_integer() {
        assert_eq!(xno_to_raw("1"), Some(RAW_PER_XNO));
        assert_eq!(xno_to_raw("0"), Some(0));
        assert_eq!(xno_to_raw("2"), Some(2 * RAW_PER_XNO));
    }

    #[test]
    fn test_xno_to_raw_decimal() {
        assert_eq!(
            xno_to_raw("0.000001"),
            Some(1_000_000_000_000_000_000_000_000)
        );
        assert_eq!(
            xno_to_raw("1.5"),
            Some(RAW_PER_XNO + 500_000_000_000_000_000_000_000_000_000)
        );
    }

    #[test]
    fn test_xno_to_raw_full_precision() {
        assert_eq!(xno_to_raw("0.000000000000000000000000000001"), Some(1));
    }

    #[test]
    fn test_xno_to_raw_invalid() {
        assert_eq!(xno_to_raw(""), None);
        assert_eq!(xno_to_raw("abc"), None);
        assert_eq!(xno_to_raw("1.2.3"), None);
    }

    #[test]
    fn test_xno_to_raw_excess_precision() {
        assert_eq!(xno_to_raw("0.0000000000000000000000000000001"), None);
    }

    #[test]
    fn test_raw_to_xno() {
        assert_eq!(raw_to_xno(RAW_PER_XNO), "1.0");
        assert_eq!(raw_to_xno(0), "0.0");
        assert_eq!(raw_to_xno(1_000_000_000_000_000_000_000_000), "0.000001");
        assert_eq!(raw_to_xno(1), "0.000000000000000000000000000001");
    }

    #[test]
    fn test_raw_to_xno_roundtrip() {
        let amounts = [
            0u128,
            1,
            RAW_PER_XNO,
            1_000_000_000_000_000_000_000_000,
            133_700_000_000_000_000_000_000_000_000,
        ];
        for &raw in &amounts {
            let xno = raw_to_xno(raw);
            let back = xno_to_raw(&xno).unwrap();
            assert_eq!(raw, back, "roundtrip failed for raw={raw}");
        }
    }
}
