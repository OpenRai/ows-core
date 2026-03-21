use base64::{engine::general_purpose::STANDARD as B64, Engine};

use crate::chains::{self, ChainMapping};
use crate::error::{PayError, PayErrorCode};
use crate::types::{
    Eip3009Authorization, Eip3009Payload, PayResult, PaymentInfo, PaymentPayload,
    PaymentRequirements, Protocol, X402Response,
};
use crate::wallet::WalletAccess;

const HEADER_PAYMENT_REQUIRED: &str = "x-payment-required";
const HEADER_PAYMENT: &str = "X-PAYMENT";

/// Handle x402 payment for a 402 response we already received.
pub(crate) async fn handle_x402(
    wallet: &dyn WalletAccess,
    url: &str,
    method: &str,
    req_body: Option<&str>,
    resp_headers: &reqwest::header::HeaderMap,
    body_402: &str,
) -> Result<PayResult, PayError> {
    let requirements = parse_requirements(resp_headers, body_402)?;
    let (req, chain) = pick_payment_option(&requirements)?;

    let account = wallet.evm_account()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let valid_after = now.saturating_sub(5);
    let valid_before = now + req.max_timeout_seconds;

    let mut nonce_bytes = [0u8; 32];
    getrandom::getrandom(&mut nonce_bytes)
        .map_err(|e| PayError::new(PayErrorCode::SigningFailed, format!("rng: {e}")))?;
    let nonce_hex = format!("0x{}", hex::encode(nonce_bytes));

    let token_name = req
        .extra
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("USD Coin");
    let token_version = req
        .extra
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("2");

    let chain_id_num: u64 = chain
        .caip2
        .split(':')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            PayError::new(
                PayErrorCode::ProtocolMalformed,
                format!("bad CAIP-2: {}", chain.caip2),
            )
        })?;

    let typed_data_json = serde_json::json!({
        "types": {
            "EIP712Domain": [
                { "name": "name", "type": "string" },
                { "name": "version", "type": "string" },
                { "name": "chainId", "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ],
            "TransferWithAuthorization": [
                { "name": "from", "type": "address" },
                { "name": "to", "type": "address" },
                { "name": "value", "type": "uint256" },
                { "name": "validAfter", "type": "uint256" },
                { "name": "validBefore", "type": "uint256" },
                { "name": "nonce", "type": "bytes32" }
            ]
        },
        "primaryType": "TransferWithAuthorization",
        "domain": {
            "name": token_name,
            "version": token_version,
            "chainId": chain_id_num.to_string(),
            "verifyingContract": req.asset
        },
        "message": {
            "from": account.address,
            "to": req.pay_to,
            "value": req.amount,
            "validAfter": valid_after.to_string(),
            "validBefore": valid_before.to_string(),
            "nonce": nonce_hex.clone()
        }
    })
    .to_string();

    let sig = wallet.sign_typed_data(chain.ows_chain, &typed_data_json)?;

    let payload = PaymentPayload {
        x402_version: 1,
        scheme: "exact".into(),
        network: req.network.clone(),
        payload: Eip3009Payload {
            signature: sig.signature,
            authorization: Eip3009Authorization {
                from: account.address,
                to: req.pay_to.clone(),
                value: req.amount.clone(),
                valid_after: valid_after.to_string(),
                valid_before: valid_before.to_string(),
                nonce: nonce_hex,
            },
        },
    };

    let payload_json = serde_json::to_string(&payload)?;
    let payload_b64 = B64.encode(payload_json.as_bytes());
    let amount_display = crate::discovery::format_usdc(&req.amount);

    let client = reqwest::Client::new();
    let retry = build_request(&client, url, method, req_body, Some(&payload_b64))
        .send()
        .await?;

    let status = retry.status().as_u16();
    let response_body = retry.text().await.unwrap_or_default();

    Ok(PayResult {
        protocol: Protocol::X402,
        status,
        body: response_body,
        payment: Some(PaymentInfo {
            amount: amount_display,
            network: chain.name.to_string(),
            token: "USDC".to_string(),
        }),
    })
}

fn parse_requirements(
    headers: &reqwest::header::HeaderMap,
    body_text: &str,
) -> Result<Vec<PaymentRequirements>, PayError> {
    if let Some(header_val) = headers.get(HEADER_PAYMENT_REQUIRED) {
        if let Ok(header_str) = header_val.to_str() {
            if let Ok(decoded) = B64.decode(header_str) {
                if let Ok(parsed) = serde_json::from_slice::<X402Response>(&decoded) {
                    if !parsed.accepts.is_empty() {
                        return Ok(parsed.accepts);
                    }
                }
            }
        }
    }

    let parsed: X402Response = serde_json::from_str(body_text).map_err(|e| {
        PayError::new(
            PayErrorCode::ProtocolMalformed,
            format!("failed to parse x402 402 response: {e}"),
        )
    })?;

    if parsed.accepts.is_empty() {
        return Err(PayError::new(
            PayErrorCode::ProtocolMalformed,
            "402 response has empty accepts",
        ));
    }

    Ok(parsed.accepts)
}

fn pick_payment_option(
    requirements: &[PaymentRequirements],
) -> Result<(&PaymentRequirements, &'static ChainMapping), PayError> {
    for req in requirements {
        if req.scheme != "exact" {
            continue;
        }
        if let Some(chain) =
            chains::chain_by_caip2(&req.network).or_else(|| chains::chain_by_name(&req.network))
        {
            return Ok((req, chain));
        }
    }

    let networks: Vec<_> = requirements.iter().map(|r| r.network.as_str()).collect();
    Err(PayError::new(
        PayErrorCode::UnsupportedChain,
        format!("no supported EVM chain in 402 response (networks: {networks:?})"),
    ))
}

pub(crate) fn build_request(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    body: Option<&str>,
    payment_header: Option<&str>,
) -> reqwest::RequestBuilder {
    let mut req = match method.to_uppercase().as_str() {
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        _ => client.get(url),
    };

    if let Some(b) = body {
        req = req
            .header("content-type", "application/json")
            .body(b.to_string());
    }

    if let Some(payment) = payment_header {
        req = req.header(HEADER_PAYMENT, payment);
    }

    req
}
