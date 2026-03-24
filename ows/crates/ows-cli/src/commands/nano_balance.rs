use crate::CliError;

/// `ows nano balance --wallet <name> [--index 0]`
///
/// Queries the balance of a Nano account.
pub fn run(wallet_name: &str, index: u32, json_output: bool) -> Result<(), CliError> {
    let chain = ows_core::ChainType::Nano;
    let key = super::resolve_signing_key(wallet_name, chain, index)?;

    let rpc_url = super::nano_send::resolve_nano_rpc()?;

    let result = ows_lib::nano_balance(key.expose(), &rpc_url)?;

    if json_output {
        let obj = serde_json::json!({
            "address": result.address,
            "balance_raw": result.balance_raw,
            "balance_xno": result.balance_xno,
            "pending_count": result.pending_count,
            "frontier": result.frontier,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        println!("{} XNO", result.balance_xno);
        eprintln!("Address: {}", result.address);
        if result.pending_count > 0 {
            eprintln!(
                "{} pending block(s) — run `ows nano receive` to claim",
                result.pending_count
            );
        }
    }

    Ok(())
}
