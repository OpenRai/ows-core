//! Nano RPC helpers (account_info, work_generate, process).
//!
//! Uses `curl` for HTTP, consistent with the rest of ows-lib (no added HTTP deps).
//!
//! PoW is local-first when recommended (via `nano-rspow`), falling back to remote RPC.
//! Control via `NANO_WORK_MODE` env: `"auto"` (default), `"local"`, `"remote"`, `"retune"`.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PowMode {
    /// Local only — fail if local PoW can't produce a result.
    Local,
    /// Remote only — skip local, go straight to RPC.
    Remote,
    /// Local first, remote fallback on failure.
    Auto,
    /// Clear local tuning cache, then behave like auto.
    Retune,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPowRecommendation {
    Skip,
    CheckCachedRecommendation,
    ClearCacheAndCheckRecommendation,
}

fn pow_mode() -> PowMode {
    pow_mode_from_env(std::env::var("NANO_WORK_MODE").ok().as_deref())
}

fn pow_mode_from_env(value: Option<&str>) -> PowMode {
    match value {
        Some("local") => PowMode::Local,
        Some("remote") => PowMode::Remote,
        Some("retune") => PowMode::Retune,
        _ => PowMode::Auto,
    }
}

fn local_pow_recommendation_for_mode(mode: PowMode) -> LocalPowRecommendation {
    match mode {
        PowMode::Auto => LocalPowRecommendation::CheckCachedRecommendation,
        PowMode::Retune => LocalPowRecommendation::ClearCacheAndCheckRecommendation,
        PowMode::Local | PowMode::Remote => LocalPowRecommendation::Skip,
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

/// Returns whether OWS should prefer local Nano PoW on this machine.
///
/// With the `local-pow` feature enabled this delegates to `nano-rspow` and may
/// use its persistent tuning cache. Without local PoW support, it always returns
/// `false`.
#[cfg(feature = "local-pow")]
fn local_pow_recommended() -> bool {
    nano_rspow::recommend_local_pow()
}

#[cfg(not(feature = "local-pow"))]
fn local_pow_recommended() -> bool {
    false
}

#[cfg(feature = "local-pow")]
fn clear_pow_tuning_cache() -> bool {
    nano_rspow::clear_pow_tuning_cache()
}

#[cfg(not(feature = "local-pow"))]
fn clear_pow_tuning_cache() -> bool {
    false
}

fn should_try_local_pow(mode: PowMode) -> bool {
    match local_pow_recommendation_for_mode(mode) {
        LocalPowRecommendation::CheckCachedRecommendation => local_pow_recommended(),
        LocalPowRecommendation::ClearCacheAndCheckRecommendation => {
            clear_pow_tuning_cache();
            local_pow_recommended()
        }
        LocalPowRecommendation::Skip => false,
    }
}

fn parse_difficulty_hex(difficulty_hex: &str) -> Result<u64, OwsLibError> {
    let trimmed = difficulty_hex.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if hex.is_empty() || hex.len() > 16 {
        return Err(OwsLibError::InvalidInput(format!(
            "Nano PoW difficulty must be 1-16 hex digits, got {difficulty_hex:?}"
        )));
    }

    u64::from_str_radix(hex, 16).map_err(|e| {
        OwsLibError::InvalidInput(format!(
            "invalid Nano PoW difficulty {difficulty_hex:?}: {e}"
        ))
    })
}

fn configured_work_urls(rpc_url: &str) -> Vec<String> {
    parse_work_urls(rpc_url, std::env::var("NANO_WORK_URL").ok().as_deref())
}

fn parse_work_urls(rpc_url: &str, urls: Option<&str>) -> Vec<String> {
    let mut endpoints: Vec<String> = vec![rpc_url.to_string()];

    if let Some(urls) = urls {
        for url in urls.split([';', ',']) {
            let url = url.trim();
            if !url.is_empty() && !endpoints.iter().any(|endpoint| endpoint == url) {
                endpoints.push(url.to_string());
            }
        }
    }

    endpoints
}

/// Request proof-of-work using a Nano RPC difficulty hex string.
///
/// Controlled by `NANO_WORK_MODE` env var:
/// - `"auto"` (default): local when recommended, remote otherwise
/// - `"local"`: local PoW only, fail if it can't produce a result
/// - `"remote"`: remote RPC only, skip local
/// - `"retune"`: clear the local tuning cache, then behave like auto
///
/// Remote endpoints are tried in order:
/// 1. The primary `rpc_url`
/// 2. URLs from `NANO_WORK_URL` env var (semicolon- or comma-separated)
pub fn work_generate(
    rpc_url: &str,
    hash: &str,
    difficulty_hex: &str,
) -> Result<String, OwsLibError> {
    let threshold = parse_difficulty_hex(difficulty_hex)?;
    work_generate_threshold(rpc_url, hash, threshold)
}

/// Request proof-of-work using a numeric threshold.
///
/// This helper is intended for internal OWS code that already deals in
/// `nano-rspow` threshold constants.
pub fn work_generate_threshold(
    rpc_url: &str,
    hash: &str,
    threshold: u64,
) -> Result<String, OwsLibError> {
    let mode = pow_mode();
    match mode {
        PowMode::Remote => {
            return work_generate_remote(rpc_url, hash, threshold);
        }
        PowMode::Local => {
            return work_generate_local(hash, threshold);
        }
        PowMode::Auto | PowMode::Retune => {}
    }

    if should_try_local_pow(mode) {
        if let Ok(work) = work_generate_local(hash, threshold) {
            return Ok(work);
        }

        eprintln!("  Local PoW failed, falling back to remote");
    }

    work_generate_remote(rpc_url, hash, threshold)
}

/// Remote PoW fallback — tries the primary RPC then `NANO_WORK_URL` endpoints.
fn work_generate_remote(rpc_url: &str, hash: &str, threshold: u64) -> Result<String, OwsLibError> {
    let difficulty_hex = format!("{:016x}", threshold);
    let endpoints = configured_work_urls(rpc_url);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch2_send_hex_matches_expected() {
        let hex = format!("{:016x}", EPOCH2_SEND);
        assert_eq!(hex, "fffffff800000000");
    }

    #[test]
    fn epoch2_receive_hex_matches_expected() {
        let hex = format!("{:016x}", EPOCH2_RECEIVE);
        assert_eq!(hex, "fffffe0000000000");
    }

    #[test]
    fn threshold_hex_roundtrip() {
        for threshold in [EPOCH2_SEND, EPOCH2_RECEIVE] {
            let hex = format!("{:016x}", threshold);
            let decoded = u64::from_str_radix(&hex, 16).unwrap();
            assert_eq!(decoded, threshold);
        }
    }

    #[test]
    fn parse_difficulty_accepts_plain_and_prefixed_hex() {
        assert_eq!(
            parse_difficulty_hex("fffffff800000000").unwrap(),
            EPOCH2_SEND
        );
        assert_eq!(
            parse_difficulty_hex("0xfffffe0000000000").unwrap(),
            EPOCH2_RECEIVE
        );
    }

    #[test]
    fn parse_difficulty_rejects_invalid_hex() {
        assert!(parse_difficulty_hex("").is_err());
        assert!(parse_difficulty_hex("10000000000000000").is_err());
        assert!(parse_difficulty_hex("not-hex").is_err());
    }

    #[test]
    fn pow_mode_from_env_accepts_canonical_modes() {
        assert_eq!(pow_mode_from_env(Some("local")), PowMode::Local);
        assert_eq!(pow_mode_from_env(Some("remote")), PowMode::Remote);
        assert_eq!(pow_mode_from_env(Some("auto")), PowMode::Auto);
        assert_eq!(pow_mode_from_env(Some("retune")), PowMode::Retune);
        assert_eq!(pow_mode_from_env(Some("unexpected")), PowMode::Auto);
        assert_eq!(pow_mode_from_env(None), PowMode::Auto);
    }

    #[test]
    fn pow_mode_recommendation_action_matches_modes() {
        assert_eq!(
            local_pow_recommendation_for_mode(PowMode::Auto),
            LocalPowRecommendation::CheckCachedRecommendation
        );
        assert_eq!(
            local_pow_recommendation_for_mode(PowMode::Retune),
            LocalPowRecommendation::ClearCacheAndCheckRecommendation
        );
        assert_eq!(
            local_pow_recommendation_for_mode(PowMode::Local),
            LocalPowRecommendation::Skip
        );
        assert_eq!(
            local_pow_recommendation_for_mode(PowMode::Remote),
            LocalPowRecommendation::Skip
        );
    }

    #[test]
    fn parse_work_urls_accepts_semicolon_and_comma() {
        assert_eq!(
            parse_work_urls("https://primary", Some("https://a; https://b,https://c")),
            vec![
                "https://primary".to_string(),
                "https://a".to_string(),
                "https://b".to_string(),
                "https://c".to_string()
            ]
        );
    }

    #[test]
    fn parse_work_urls_skips_empty_and_duplicate_endpoints() {
        assert_eq!(
            parse_work_urls(
                "https://primary",
                Some(" ;https://a,https://primary;https://a")
            ),
            vec!["https://primary".to_string(), "https://a".to_string()]
        );
    }

    #[cfg(feature = "local-pow")]
    #[test]
    fn local_pow_known_test_vector() {
        let hash_bytes =
            hex::decode("718CC2121C3E641059BC1C2CFC45666C99E8AE922F7A807B7D07B62C995D79E2")
                .unwrap();
        let hash: [u8; 32] = hash_bytes.try_into().unwrap();
        let work = u64::from_str_radix("2bf29ef00786a6bc", 16).unwrap();
        let result = nano_rspow::work_validate(&hash, work, nano_rspow::thresholds::EPOCH1);
        assert!(result.is_valid(), "result should meet the threshold");
    }
}
