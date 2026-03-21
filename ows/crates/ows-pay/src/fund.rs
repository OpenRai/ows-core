use crate::chains::{self, ChainMapping, DEFAULT_CHAIN};
use crate::error::{PayError, PayErrorCode};
use crate::types::{
    FundResult, MoonPayBalanceRequest, MoonPayBalanceResponse, MoonPayDepositRequest,
    MoonPayDepositResponse, TokenBalance,
};

const MOONPAY_API: &str = "https://agents.moonpay.com";

/// Create a MoonPay deposit that auto-converts incoming crypto to USDC.
pub async fn fund(
    wallet_address: &str,
    chain: Option<&str>,
    token: Option<&str>,
) -> Result<FundResult, PayError> {
    let mapping = resolve_chain(chain)?;
    let token = token.unwrap_or("USDC");

    let client = reqwest::Client::new();
    let req = MoonPayDepositRequest {
        name: format!("OWS deposit ({token} on {})", mapping.name),
        wallet: wallet_address.to_string(),
        chain: mapping.moonpay_chain.to_string(),
        token: token.to_string(),
    };

    let resp = client
        .post(format!("{MOONPAY_API}/api/tools/deposit_create"))
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(PayError::new(
            PayErrorCode::HttpStatus,
            format!("MoonPay returned {status}: {body}"),
        ));
    }

    let deposit: MoonPayDepositResponse = resp.json().await?;

    Ok(FundResult {
        deposit_id: deposit.id,
        deposit_url: deposit.deposit_url,
        wallets: deposit
            .wallets
            .iter()
            .map(|w| (w.chain.clone(), w.address.clone()))
            .collect(),
        instructions: deposit.instructions,
    })
}

/// Check token balances for a wallet address via MoonPay.
pub async fn get_balances(
    wallet_address: &str,
    chain: Option<&str>,
) -> Result<Vec<TokenBalance>, PayError> {
    let mapping = resolve_chain(chain)?;
    let client = reqwest::Client::new();

    let req = MoonPayBalanceRequest {
        wallet: wallet_address.to_string(),
        chain: mapping.moonpay_chain.to_string(),
    };

    let resp = client
        .post(format!("{MOONPAY_API}/api/tools/token_balance_list"))
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(PayError::new(
            PayErrorCode::HttpStatus,
            format!("MoonPay balance returned {status}: {body}"),
        ));
    }

    let balance_resp: MoonPayBalanceResponse = resp.json().await?;
    Ok(balance_resp.items)
}

fn resolve_chain(chain: Option<&str>) -> Result<&'static ChainMapping, PayError> {
    match chain {
        Some(name) => chains::chain_by_name(name).ok_or_else(|| {
            PayError::new(
                PayErrorCode::UnsupportedChain,
                format!("unknown chain: {name}"),
            )
        }),
        None => Ok(DEFAULT_CHAIN),
    }
}
