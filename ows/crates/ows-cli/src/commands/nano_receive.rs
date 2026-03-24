use crate::CliError;

/// `ows nano receive --wallet <name> [--index 0]`
///
/// Receives all pending blocks for a Nano account.
pub fn run(wallet_name: &str, index: u32, json_output: bool) -> Result<(), CliError> {
    let chain = ows_core::ChainType::Nano;
    let key = super::resolve_signing_key(wallet_name, chain, index)?;

    let rpc_url = super::nano_send::resolve_nano_rpc()?;

    eprintln!("Checking for pending blocks...");

    let result = ows_lib::nano_receive(key.expose(), &rpc_url)?;

    if result.received.is_empty() {
        if json_output {
            let obj = serde_json::json!({
                "received": [],
                "balance_raw": result.new_balance_raw,
                "balance_xno": result.new_balance_xno,
            });
            println!("{}", serde_json::to_string_pretty(&obj)?);
        } else {
            eprintln!("No pending blocks to receive.");
            println!("{} XNO", result.new_balance_xno);
        }
        return Ok(());
    }

    if json_output {
        let blocks: Vec<serde_json::Value> = result
            .received
            .iter()
            .map(|b| {
                let amount_u128: u128 = b.amount_raw.parse().unwrap_or(0);
                serde_json::json!({
                    "block_hash": b.block_hash,
                    "amount_raw": b.amount_raw,
                    "amount_xno": ows_lib::nano_rpc::raw_to_xno(amount_u128),
                    "source": b.source,
                })
            })
            .collect();

        let obj = serde_json::json!({
            "received": blocks,
            "balance_raw": result.new_balance_raw,
            "balance_xno": result.new_balance_xno,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        for block in &result.received {
            let amount_u128: u128 = block.amount_raw.parse().unwrap_or(0);
            let amount_xno = ows_lib::nano_rpc::raw_to_xno(amount_u128);
            eprintln!(
                "  Received {} XNO from {} ({})",
                amount_xno, block.source, block.block_hash
            );
        }
        eprintln!("Received {} block(s). New balance:", result.received.len());
        println!("{} XNO", result.new_balance_xno);
    }

    Ok(())
}
