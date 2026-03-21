//! `ows-pay` — payment client for the Open Wallet Standard.
//!
//! Supports x402 and MPP protocols with automatic detection.
//!
//! ```ignore
//! let result = ows_pay::pay(&wallet, "https://api.example.com/data", "GET", None).await?;
//! let services = ows_pay::discover(None).await?;
//! ```

pub(crate) mod chains;
pub(crate) mod discovery;
pub mod error;
pub mod fund;
pub mod types;
pub mod wallet;

// Protocol implementations (internal).
mod mpp;
mod x402;

pub use error::{PayError, PayErrorCode};
pub use types::{PayResult, PaymentInfo, Protocol, Service};
pub use wallet::{EvmAccount, TypedDataSignature, WalletAccess};

/// Make an HTTP request with automatic payment handling.
///
/// Fires the request. If the server returns 402, detects the payment
/// protocol (x402 or MPP) from the response and handles payment.
pub async fn pay(
    wallet: &dyn WalletAccess,
    url: &str,
    method: &str,
    body: Option<&str>,
) -> Result<PayResult, PayError> {
    let client = reqwest::Client::new();

    // Step 1: Fire the initial request.
    let initial = x402::build_request(&client, url, method, body, None)
        .send()
        .await?;

    // Step 2: Not a 402 — return directly.
    if initial.status().as_u16() != 402 {
        let status = initial.status().as_u16();
        let text = initial.text().await.unwrap_or_default();
        return Ok(PayResult {
            protocol: Protocol::X402,
            status,
            body: text,
            payment: None,
        });
    }

    // Step 3: Got a 402. Extract headers + body for protocol detection.
    let headers = initial.headers().clone();
    let body_402 = initial.text().await.unwrap_or_default();

    // Step 4: Detect protocol from the 402 response.
    //
    // MPP: WWW-Authenticate header starting with 'Payment ' (RFC draft).
    // x402: everything else (body-based accepts array or x-payment-required header).
    //
    // We check the header strictly — not substring matching on the body,
    // which could be spoofed by any server's error text.
    let is_mpp = headers
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("Payment ") || v.starts_with("Payment,"))
        .unwrap_or(false);

    if is_mpp {
        // MPP: re-fire through the MPP SDK (it manages its own 402 flow).
        mpp::handle_mpp(wallet, url, method, body).await
    } else {
        // x402: sign and retry with the 402 data we already have.
        x402::handle_x402(wallet, url, method, body, &headers, &body_402).await
    }
}

/// Discover payable services across all protocols.
pub async fn discover(query: Option<&str>) -> Result<Vec<Service>, PayError> {
    discovery::discover_all(query).await
}
