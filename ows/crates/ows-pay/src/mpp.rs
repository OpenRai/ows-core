use crate::error::{PayError, PayErrorCode};
use crate::types::{PayResult, PaymentInfo, Protocol};
use crate::wallet::WalletAccess;

use mpp::client::{Fetch, PaymentProvider};
use mpp::protocol::core::{PaymentChallenge, PaymentCredential};
use mpp::{Address, MppError};

const TEMPO_RPC: &str = "https://rpc.tempo.xyz";

/// Handle MPP payment. The private key never leaves OWS — signing is
/// delegated through the WalletAccess trait via a custom alloy Signer.
pub(crate) async fn handle_mpp(
    wallet: &dyn WalletAccess,
    url: &str,
    method: &str,
    body: Option<&str>,
) -> Result<PayResult, PayError> {
    let account = wallet.evm_account()?;
    let address: Address = account
        .address
        .parse()
        .map_err(|e| PayError::new(PayErrorCode::SigningFailed, format!("bad address: {e}")))?;

    let provider = OwsTempoProvider {
        wallet,
        address,
        rpc_url: TEMPO_RPC.parse().unwrap(),
    };

    let client = reqwest::Client::new();
    let req = match method.to_uppercase().as_str() {
        "POST" => {
            let mut r = client.post(url);
            if let Some(b) = body {
                r = r
                    .header("content-type", "application/json")
                    .body(b.to_string());
            }
            r
        }
        "PUT" => {
            let mut r = client.put(url);
            if let Some(b) = body {
                r = r
                    .header("content-type", "application/json")
                    .body(b.to_string());
            }
            r
        }
        _ => client.get(url),
    };

    let resp = req
        .send_with_payment(&provider)
        .await
        .map_err(|e| PayError::new(PayErrorCode::HttpTransport, format!("MPP request: {e}")))?;

    let status = resp.status().as_u16();
    let response_body = resp.text().await.unwrap_or_default();

    Ok(PayResult {
        protocol: Protocol::Mpp,
        status,
        body: response_body,
        payment: Some(PaymentInfo {
            amount: String::new(),
            network: "Tempo".to_string(),
            token: "pathUSD".to_string(),
        }),
    })
}

// ---------------------------------------------------------------------------
// OWS-backed alloy Signer (private key never leaves OWS)
// ---------------------------------------------------------------------------

/// An alloy Signer that delegates hash signing to OWS's WalletAccess.
/// The private key never leaves OWS — only the 32-byte hash goes out,
/// only the 65-byte signature comes back.
struct OwsSigner<'a> {
    wallet: &'a dyn WalletAccess,
    address: Address,
}

impl Clone for OwsSigner<'_> {
    fn clone(&self) -> Self {
        Self {
            wallet: self.wallet,
            address: self.address,
        }
    }
}

#[async_trait::async_trait]
impl alloy::signers::Signer for OwsSigner<'_> {
    async fn sign_hash(
        &self,
        hash: &alloy::primitives::B256,
    ) -> alloy::signers::Result<alloy::primitives::Signature> {
        let sig_bytes = self
            .wallet
            .sign_hash(hash.as_slice())
            .map_err(|e| alloy::signers::Error::other(e.message))?;

        if sig_bytes.len() != 65 {
            return Err(alloy::signers::Error::other(format!(
                "expected 65-byte signature, got {}",
                sig_bytes.len()
            )));
        }

        // Parse r (32) || s (32) || v (1)
        let r = alloy::primitives::U256::from_be_slice(&sig_bytes[..32]);
        let s = alloy::primitives::U256::from_be_slice(&sig_bytes[32..64]);
        let v = sig_bytes[64];

        // v must be 0, 1, 27, or 28. Normalize to parity bool.
        let y_parity = match v {
            0 | 27 => false,
            1 | 28 => true,
            _ => {
                return Err(alloy::signers::Error::other(format!(
                    "invalid recovery id: {v} (expected 0, 1, 27, or 28)"
                )));
            }
        };

        Ok(alloy::primitives::Signature::new(r, s, y_parity))
    }

    fn address(&self) -> Address {
        self.address
    }

    fn chain_id(&self) -> Option<alloy::primitives::ChainId> {
        None
    }

    fn set_chain_id(&mut self, _chain_id: Option<alloy::primitives::ChainId>) {}
}

// ---------------------------------------------------------------------------
// OWS-backed PaymentProvider
// ---------------------------------------------------------------------------

struct OwsTempoProvider<'a> {
    wallet: &'a dyn WalletAccess,
    address: Address,
    rpc_url: reqwest::Url,
}

impl Clone for OwsTempoProvider<'_> {
    fn clone(&self) -> Self {
        Self {
            wallet: self.wallet,
            address: self.address,
            rpc_url: self.rpc_url.clone(),
        }
    }
}

impl<'a> PaymentProvider for OwsTempoProvider<'a> {
    fn supports(&self, method: &str, intent: &str) -> bool {
        method == "tempo" && intent == "charge"
    }

    async fn pay(&self, challenge: &PaymentChallenge) -> Result<PaymentCredential, MppError> {
        use mpp::client::tempo::charge::{SignOptions, TempoCharge};

        let signer = OwsSigner {
            wallet: self.wallet,
            address: self.address,
        };

        let mut charge = TempoCharge::from_challenge(challenge)?;

        if charge.memo().is_none() {
            let memo = mpp::tempo::attribution::encode(&challenge.realm, None);
            charge = charge.with_memo(memo);
        }

        let options = SignOptions {
            rpc_url: Some(self.rpc_url.to_string()),
            ..Default::default()
        };

        let signed = charge.sign_with_options(&signer, options).await?;
        Ok(signed.into_credential())
    }
}
