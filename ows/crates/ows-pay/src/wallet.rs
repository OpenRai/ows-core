use crate::error::PayError;

/// Information about a wallet's EVM account.
#[derive(Debug, Clone)]
pub struct EvmAccount {
    /// Checksummed EVM address (0x...).
    pub address: String,
}

/// Result of signing EIP-712 typed data.
#[derive(Debug, Clone)]
pub struct TypedDataSignature {
    /// Hex-encoded signature with 0x prefix.
    pub signature: String,
}

/// Trait abstracting wallet access for payment operations.
///
/// Implement this to provide wallet functionality without coupling to ows-lib.
/// The private key NEVER leaves the implementation — all signing happens
/// inside the wallet.
pub trait WalletAccess: Send + Sync {
    /// Get the EVM account info for this wallet.
    fn evm_account(&self) -> Result<EvmAccount, PayError>;

    /// Sign EIP-712 typed data. Returns the hex signature with 0x prefix.
    /// Used by x402 (TransferWithAuthorization).
    fn sign_typed_data(
        &self,
        chain: &str,
        typed_data_json: &str,
    ) -> Result<TypedDataSignature, PayError>;

    /// Sign a raw 32-byte hash with the EVM key (secp256k1).
    /// Returns 65 bytes: r (32) || s (32) || v (1).
    /// Used by MPP (Tempo transaction signing via alloy's Signer trait).
    fn sign_hash(&self, hash: &[u8]) -> Result<Vec<u8>, PayError>;
}
