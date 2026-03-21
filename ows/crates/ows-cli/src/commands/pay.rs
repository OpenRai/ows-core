use crate::commands::read_passphrase;
use crate::CliError;

/// Concrete WalletAccess backed by ows-lib.
struct OwsLibWallet {
    wallet_name: String,
    passphrase: String,
}

impl ows_pay::WalletAccess for OwsLibWallet {
    fn evm_account(&self) -> Result<ows_pay::EvmAccount, ows_pay::PayError> {
        let info = ows_lib::get_wallet(&self.wallet_name, None).map_err(|e| {
            ows_pay::PayError::new(ows_pay::PayErrorCode::WalletNotFound, e.to_string())
        })?;
        let acct = info
            .accounts
            .iter()
            .find(|a| a.chain_id.starts_with("eip155:"))
            .ok_or_else(|| {
                ows_pay::PayError::new(ows_pay::PayErrorCode::WalletNotFound, "no EVM account")
            })?;
        Ok(ows_pay::EvmAccount {
            address: acct.address.clone(),
        })
    }

    fn sign_typed_data(
        &self,
        chain: &str,
        typed_data_json: &str,
    ) -> Result<ows_pay::TypedDataSignature, ows_pay::PayError> {
        let result = ows_lib::sign_typed_data(
            &self.wallet_name,
            chain,
            typed_data_json,
            Some(&self.passphrase),
            None,
            None,
        )
        .map_err(|e| ows_pay::PayError::new(ows_pay::PayErrorCode::SigningFailed, e.to_string()))?;
        Ok(ows_pay::TypedDataSignature {
            signature: format!("0x{}", result.signature),
        })
    }

    fn sign_hash(&self, hash: &[u8]) -> Result<Vec<u8>, ows_pay::PayError> {
        // Decrypt the EVM key, sign the hash, return 65-byte signature.
        // The key is decrypted in-process and zeroized after signing.
        let key = ows_lib::decrypt_signing_key(
            &self.wallet_name,
            ows_core::ChainType::Evm,
            &self.passphrase,
            None,
            None,
        )
        .map_err(|e| ows_pay::PayError::new(ows_pay::PayErrorCode::SigningFailed, e.to_string()))?;

        let signer = ows_signer::signer_for_chain(ows_core::ChainType::Evm);
        let output = signer.sign(key.expose(), hash).map_err(|e| {
            ows_pay::PayError::new(ows_pay::PayErrorCode::SigningFailed, e.to_string())
        })?;

        // EVM sign() returns 65 bytes: r(32) || s(32) || v(1).
        // Normalize v to legacy format (27 or 28).
        let mut sig = output.signature;
        if sig.len() == 65 {
            match sig[64] {
                0 | 1 => sig[64] += 27, // raw recovery id → legacy v
                27 | 28 => {}           // already legacy format
                v => {
                    return Err(ows_pay::PayError::new(
                        ows_pay::PayErrorCode::SigningFailed,
                        format!("invalid recovery id from signer: {v}"),
                    ));
                }
            }
        }
        Ok(sig)
    }
}

/// `ows pay request <url> --wallet <name> [--method GET] [--body '{}']`
pub fn run(
    url: &str,
    wallet_name: &str,
    method: &str,
    body: Option<&str>,
    skip_passphrase: bool,
) -> Result<(), CliError> {
    let passphrase = if skip_passphrase {
        String::new()
    } else {
        read_passphrase().to_string()
    };

    let wallet = OwsLibWallet {
        wallet_name: wallet_name.to_string(),
        passphrase,
    };

    let rt =
        tokio::runtime::Runtime::new().map_err(|e| CliError::InvalidArgs(format!("tokio: {e}")))?;

    let result = rt.block_on(ows_pay::pay(&wallet, url, method, body))?;

    if let Some(ref payment) = result.payment {
        if !payment.amount.is_empty() {
            eprintln!(
                "Paid {} on {} via {}",
                payment.amount, payment.network, result.protocol
            );
        } else {
            eprintln!("Paid via {}", result.protocol);
        }
    }

    if result.status >= 400 {
        eprintln!("HTTP {}", result.status);
    }

    println!("{}", result.body);
    Ok(())
}

/// `ows pay discover [--query <search>]`
pub fn discover(query: Option<&str>) -> Result<(), CliError> {
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| CliError::InvalidArgs(format!("tokio: {e}")))?;

    let services = rt.block_on(ows_pay::discover(query))?;

    if services.is_empty() {
        eprintln!("No services found.");
        return Ok(());
    }

    let (mpp, x402): (Vec<_>, Vec<_>) = services
        .iter()
        .partition(|s| s.protocol == ows_pay::Protocol::Mpp);

    if !mpp.is_empty() {
        eprintln!("MPP ({} services):\n", mpp.len());
        for svc in &mpp {
            println!(
                "  {:<25} {:>8}  [{}]",
                svc.name,
                svc.price,
                svc.tags.join(", ")
            );
            println!("  {:25} {}", "", svc.description);
            println!("  {:25} {}", "", svc.url);
            println!();
        }
    }

    if !x402.is_empty() {
        eprintln!("x402 ({} services):\n", x402.len());
        for svc in &x402 {
            println!(
                "  {:>8}  {:<8}  {}",
                svc.price, svc.network, svc.description
            );
            println!("  {:>8}  {:8}  {}", "", "", svc.url);
            println!();
        }
    }

    Ok(())
}
