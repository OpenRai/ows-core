use crate::CliError;

/// `ows nano send --wallet <name> --to <nano_addr> --amount <xno> [--index 0]`
///
/// Builds a Nano send block, signs, generates PoW, and broadcasts.
pub fn run(
    wallet_name: &str,
    to: &str,
    amount: &str,
    index: u32,
    json_output: bool,
) -> Result<(), CliError> {
    let chain = ows_core::ChainType::Nano;
    let key = super::resolve_signing_key(wallet_name, chain, index)?;

    // Resolve RPC URL
    let rpc_url = resolve_nano_rpc()?;

    // Parse amount
    let amount_raw = ows_lib::nano_rpc::xno_to_raw(amount).ok_or_else(|| {
        CliError::InvalidArgs(format!(
            "invalid XNO amount: '{amount}' (use decimal like '1.5' or '0.001')"
        ))
    })?;

    if amount_raw == 0 {
        return Err(CliError::InvalidArgs(
            "amount must be greater than 0".into(),
        ));
    }

    eprintln!("Sending {amount} XNO to {to}...");

    let result = ows_lib::nano_send(key.expose(), to, amount_raw, &rpc_url)?;

    if json_output {
        let obj = serde_json::json!({
            "block_hash": result.block_hash,
            "amount_raw": result.amount_raw,
            "amount_xno": amount,
            "to": to,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        println!("{}", result.block_hash);
    }

    Ok(())
}

/// Resolve the Nano RPC URL from env var or default config.
pub(crate) fn resolve_nano_rpc() -> Result<String, CliError> {
    if let Ok(url) = std::env::var("NANO_RPC_URL") {
        let url = url.split(',').next().unwrap_or("").trim().to_string();
        if !url.is_empty() {
            return Ok(url);
        }
    }

    let config = ows_core::Config::load_or_default();
    let defaults = ows_core::Config::default_rpc();

    // Try exact chain_id
    if let Some(url) = config.rpc.get("nano:mainnet") {
        return Ok(url.clone());
    }
    if let Some(url) = defaults.get("nano:mainnet") {
        return Ok(url.clone());
    }

    // Fallback to namespace match
    for (key, url) in &config.rpc {
        if key.starts_with("nano") {
            return Ok(url.clone());
        }
    }
    for (key, url) in &defaults {
        if key.starts_with("nano") {
            return Ok(url.clone());
        }
    }

    Err(CliError::InvalidArgs(
        "no Nano RPC URL configured (set NANO_RPC_URL or configure nano:mainnet in ows config)"
            .into(),
    ))
}
