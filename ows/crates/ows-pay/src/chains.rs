/// Chain mapping between OWS, MoonPay, and x402 identifiers.
///
/// This is the single source of truth for chain compatibility across the three systems.

/// A supported chain with identifiers across all systems.
#[derive(Debug, Clone)]
pub struct ChainMapping {
    /// Human-readable name.
    pub name: &'static str,
    /// OWS chain string (e.g. "ethereum", "base").
    pub ows_chain: &'static str,
    /// CAIP-2 identifier used by x402 (e.g. "eip155:8453").
    pub caip2: &'static str,
    /// MoonPay chain name for balance/deposit queries (e.g. "base").
    pub moonpay_chain: &'static str,
}

/// All chains where OWS + MoonPay + x402 overlap (the "golden path" chains).
pub const SUPPORTED_CHAINS: &[ChainMapping] = &[
    ChainMapping {
        name: "Base",
        ows_chain: "base",
        caip2: "eip155:8453",
        moonpay_chain: "base",
    },
    ChainMapping {
        name: "Ethereum",
        ows_chain: "ethereum",
        caip2: "eip155:1",
        moonpay_chain: "ethereum",
    },
    ChainMapping {
        name: "Polygon",
        ows_chain: "polygon",
        caip2: "eip155:137",
        moonpay_chain: "polygon",
    },
    ChainMapping {
        name: "Arbitrum",
        ows_chain: "arbitrum",
        caip2: "eip155:42161",
        moonpay_chain: "arbitrum",
    },
    ChainMapping {
        name: "Optimism",
        ows_chain: "optimism",
        caip2: "eip155:10",
        moonpay_chain: "optimism",
    },
];

/// Testnet chains (for development / demos).
pub const TESTNET_CHAINS: &[ChainMapping] = &[ChainMapping {
    name: "Base Sepolia",
    ows_chain: "base-sepolia",
    caip2: "eip155:84532",
    moonpay_chain: "base-sepolia",
}];

/// Default chain for payments (Base — lowest gas, broadest x402 support).
pub const DEFAULT_CHAIN: &ChainMapping = &SUPPORTED_CHAINS[0]; // Base

/// Look up a chain by its CAIP-2 network identifier (from an x402 402 response).
pub fn chain_by_caip2(caip2: &str) -> Option<&'static ChainMapping> {
    SUPPORTED_CHAINS
        .iter()
        .chain(TESTNET_CHAINS.iter())
        .find(|c| c.caip2 == caip2)
}

/// Look up a chain by human-readable name (case-insensitive).
pub fn chain_by_name(name: &str) -> Option<&'static ChainMapping> {
    let lower = name.to_lowercase();
    SUPPORTED_CHAINS
        .iter()
        .chain(TESTNET_CHAINS.iter())
        .find(|c| {
            c.name.to_lowercase() == lower || c.ows_chain == lower || c.moonpay_chain == lower
        })
}
