use crate::error::{PayError, PayErrorCode};
use crate::types::{
    DiscoveredService, DiscoveryResponse, MppService, MppServicesResponse, Protocol, Service,
};

const CDP_DISCOVERY_URL: &str = "https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources";
const MPP_SERVICES_URL: &str = "https://mpp.dev/api/services";

const TESTNETS: &[&str] = &[
    "base-sepolia",
    "eip155:84532",
    "eip155:11155111",
    "solana-devnet",
];

// ===========================================================================
// Unified discovery (public API)
// ===========================================================================

/// Discover payable services across all protocols.
///
/// Fetches x402 and MPP directories in parallel, filters testnets,
/// and returns a unified list.
pub async fn discover_all(query: Option<&str>) -> Result<Vec<Service>, PayError> {
    let (x402_result, mpp_result) = tokio::join!(
        async {
            match query {
                Some(q) => search_x402(q).await,
                None => fetch_x402(Some(100), None).await,
            }
        },
        async {
            match query {
                Some(q) => search_mpp(q).await,
                None => fetch_mpp().await,
            }
        }
    );

    let mut services = Vec::new();

    // Convert MPP services (higher quality, show first).
    for svc in mpp_result.unwrap_or_default() {
        services.push(Service {
            protocol: Protocol::Mpp,
            name: svc.name.clone(),
            url: svc.service_url.clone(),
            description: truncate(&svc.description, 80),
            price: cheapest_mpp_price(&svc),
            network: "Tempo".to_string(),
            tags: svc.categories,
        });
    }

    // Convert x402 services (filter testnets).
    for svc in x402_result.unwrap_or_default() {
        let accept = match svc.accepts.first() {
            Some(a) => a,
            None => continue,
        };

        let is_testnet = TESTNETS.iter().any(|t| accept.network.contains(t));
        if is_testnet {
            continue;
        }

        let desc = accept
            .description
            .as_deref()
            .or_else(|| svc.metadata.as_ref().and_then(|m| m.description.as_deref()))
            .unwrap_or("");

        services.push(Service {
            protocol: Protocol::X402,
            name: svc.resource.clone(),
            url: svc.resource,
            description: truncate(desc, 80),
            price: format_usdc(&accept.amount),
            network: accept.network.clone(),
            tags: vec![],
        });
    }

    Ok(services)
}

// ===========================================================================
// x402 fetching (internal)
// ===========================================================================

pub(crate) async fn fetch_x402(
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<Vec<DiscoveredService>, PayError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(CDP_DISCOVERY_URL)
        .query(&[
            ("limit", limit.unwrap_or(100).to_string()),
            ("offset", offset.unwrap_or(0).to_string()),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(PayError::new(
            PayErrorCode::DiscoveryFailed,
            format!("x402 discovery returned {status}: {body}"),
        ));
    }

    let body: DiscoveryResponse = resp.json().await.map_err(|e| {
        PayError::new(
            PayErrorCode::DiscoveryFailed,
            format!("failed to parse x402 discovery: {e}"),
        )
    })?;

    Ok(body.items)
}

async fn search_x402(query: &str) -> Result<Vec<DiscoveredService>, PayError> {
    let all = fetch_x402(Some(100), None).await?;
    let q = query.to_lowercase();

    Ok(all
        .into_iter()
        .filter(|s| {
            let url_match = s.resource.to_lowercase().contains(&q);
            let accepts_desc = s
                .accepts
                .first()
                .and_then(|a| a.description.as_ref())
                .map(|d| d.to_lowercase().contains(&q))
                .unwrap_or(false);
            let meta_desc = s
                .metadata
                .as_ref()
                .and_then(|m| m.description.as_ref())
                .map(|d| d.to_lowercase().contains(&q))
                .unwrap_or(false);
            url_match || accepts_desc || meta_desc
        })
        .collect())
}

// ===========================================================================
// MPP fetching (internal)
// ===========================================================================

pub(crate) async fn fetch_mpp() -> Result<Vec<MppService>, PayError> {
    let client = reqwest::Client::new();
    let resp = client.get(MPP_SERVICES_URL).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(PayError::new(
            PayErrorCode::DiscoveryFailed,
            format!("MPP discovery returned {status}: {body}"),
        ));
    }

    let body: MppServicesResponse = resp.json().await.map_err(|e| {
        PayError::new(
            PayErrorCode::DiscoveryFailed,
            format!("failed to parse MPP discovery: {e}"),
        )
    })?;

    Ok(body.services)
}

async fn search_mpp(query: &str) -> Result<Vec<MppService>, PayError> {
    let all = fetch_mpp().await?;
    let q = query.to_lowercase();

    Ok(all
        .into_iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&q)
                || s.description.to_lowercase().contains(&q)
                || s.categories.iter().any(|c| c.to_lowercase().contains(&q))
                || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect())
}

// ===========================================================================
// Formatting helpers
// ===========================================================================

pub(crate) fn format_usdc(amount_str: &str) -> String {
    let amount: u128 = amount_str.parse().unwrap_or(0);
    let whole = amount / 1_000_000;
    let frac = amount % 1_000_000;
    let frac_str = format!("{frac:06}");
    let trimmed = frac_str.trim_end_matches('0');
    let trimmed = if trimmed.is_empty() { "00" } else { trimmed };
    format!("${whole}.{trimmed}")
}

pub(crate) fn format_mpp_amount(amount_str: &str, decimals: u8) -> String {
    let amount: u128 = amount_str.parse().unwrap_or(0);
    let divisor = 10u128.pow(decimals as u32);
    let whole = amount / divisor;
    let frac = amount % divisor;
    let frac_str = format!("{frac:0>width$}", width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    let trimmed = if trimmed.is_empty() { "00" } else { trimmed };
    format!("${whole}.{trimmed}")
}

fn cheapest_mpp_price(svc: &MppService) -> String {
    svc.endpoints
        .iter()
        .filter_map(|e| e.payment.as_ref())
        .filter_map(|p| {
            let amt_str = p.amount.as_deref()?;
            let decimals = p.decimals.unwrap_or(6);
            let amt: u128 = amt_str.parse().ok()?;
            Some((amt, decimals))
        })
        .min_by_key(|(amt, _)| *amt)
        .map(|(amt, dec)| format_mpp_amount(&amt.to_string(), dec))
        .unwrap_or_else(|| "free".into())
}

fn truncate(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or("");
    if first_line.len() > max {
        format!("{}...", &first_line[..max.saturating_sub(3)])
    } else {
        first_line.to_string()
    }
}
