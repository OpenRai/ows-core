# Feature state: indexed wallet-address derivation

**Repository state:** `feat/wallet-address-derivation` (uncommitted work).

## Decision

Treat indexed address derivation as a generic OWS capability. Do not create a
Nano-only derivation API.

`ows_lib::derive_wallet_address` accepts a wallet, a registered chain, an
optional credential, an optional account index, and an optional vault path. It
selects the chain signer through the existing chain registry. The signer then
selects its derivation path and encodes the derived public key as an address.

## Current surface

| Surface | State | Contract |
| --- | --- | --- |
| Rust | Implemented | `derive_wallet_address(wallet, chain, credential, index, vault_path)` returns an address without exporting a mnemonic or private key. |
| Node native binding | Implemented | `deriveWalletAddress(wallet, chain, credential?, index?, vaultPath?)`. |
| CLI | Implemented | `ows wallet derive-address --wallet <name-or-id> --chain <chain> --index <n> [--vault <path>]`. |
| API-token mode | Implemented | Verify token expiry and wallet scope before derivation. Transaction policies do not apply because derivation does not sign. |

## Applicability

The capability applies to every chain family currently registered by OWS:
EVM, Solana, Bitcoin, Cosmos, Tron, TON, Spark, Filecoin, Sui, XRPL, Nano, and
NEAR. It also applies to each registered network identifier that resolves to
one of those families.

It does not make transaction construction, broadcast, confirmation, fee
selection, or recovery generic. Those remain chain- and network-specific.

## Nano-specific behavior

Nano uses the existing generic flow but its signer supplies the Nano details:

- Derivation path: `m/44'/165'/<index>'`.
- Key scheme: Nano's Blake2b-based Ed25519 variant.
- Address encoding: `nano_...` custom base32 with a Nano checksum.

Nano is therefore not special at the address-derivation boundary. It is special
at the state-block, proof-of-work, frontier, confirmation, and retry boundaries.

## Verification recorded

- OWS library test: an API token derives only from its scoped wallet.
- Node native-binding test: indexes `0` and `7` derive stable, distinct Nano addresses.
- CLI capture: `ows wallet derive-address --help` and an isolated-vault Nano index-7 derivation completed successfully.
- CLI unit suite: `cargo test -p ows-cli` passed with 6 tests.

## Follow-up work

1. Add an equivalent indexed-derivation method to the Python binding.
2. Publish coordinated OWS and Node-binding releases before consumers require this API.
3. Add cross-family vectors for the generic API, including the default index and a nonzero index.
4. Keep chain-specific transaction lifecycle behavior outside this address-derivation feature.

## Sources in this repository

- `ows/crates/ows-lib/src/ops.rs`: generic derivation entry point.
- `ows/crates/ows-lib/src/key_ops.rs`: scoped API-token derivation.
- `ows/crates/ows-core/src/chain.rs`: registered chain families.
- `ows/crates/ows-signer/src/traits.rs`: signer contract.
- `ows/crates/ows-signer/src/chains/nano.rs`: Nano-specific path and address encoding.
