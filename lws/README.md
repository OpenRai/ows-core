# LWS — Local Wallet Standard

Rust implementation of the [Local Wallet Standard](https://localwalletstandard.org) for secure, local-first crypto wallet management.

## Commands

| Command | Description |
|---------|-------------|
| `lws wallet create` | Create a new wallet (generates mnemonic, encrypts, saves to vault) |
| `lws wallet list` | List all saved wallets in the vault |
| `lws wallet info` | Show vault path and supported chains |
| `lws sign message` | Sign a message using a vault wallet with chain-specific formatting |
| `lws sign tx` | Sign a raw transaction using a vault wallet |
| `lws mnemonic generate` | Generate a new BIP-39 mnemonic phrase |
| `lws mnemonic derive` | Derive an address from a mnemonic (via env or stdin) |
| `lws update` | Update lws to the latest version |
| `lws uninstall` | Remove lws from the system |

## Quick Install

```bash
curl -fsSL https://openwallet.sh/install.sh | bash
```

Or clone and install locally:

```bash
git clone https://github.com/dawnlabsai/lws.git
cd lws
./lws/install.sh
```

## Manual Build

Requires [Rust](https://rustup.rs) 1.70+.

```bash
cd lws
cargo build --workspace --release
cargo test --workspace
```

## Crates

| Crate | Description |
|-------|-------------|
| `lws-core` | Types, CAIP-2/10 parsing, errors, config. Zero crypto dependencies. |
| `lws-signer` | ChainSigner trait, HD derivation, address derivation for EVM, Solana, Bitcoin, Cosmos, and Tron. |

## Supported Chains

- **EVM** (Ethereum, Polygon, etc.) — secp256k1, EIP-55 addresses, EIP-191 message signing
- **Solana** — Ed25519, base58 addresses
- **Bitcoin** — secp256k1, BIP-84 native segwit (bech32)
- **Cosmos** — secp256k1, bech32 addresses (configurable HRP)
- **Tron** — secp256k1, base58check addresses

## License

See repository root for license information.
