# Local Wallet Standard (LWS)

A specification and reference implementation for secure, local-first crypto wallet management — designed for AI agents.

## Motivation

AI agents increasingly need to interact with blockchains: signing transactions, managing accounts, and moving value across chains. Existing wallet infrastructure was built for humans clicking buttons in browser extensions, not for programmatic agents operating autonomously.

LWS addresses this gap. It defines a minimal, chain-agnostic standard for wallet operations where:

- **Private keys never leave the local machine.** Keys are stored in encrypted Ethereum Keystore v3 format with strict filesystem permissions — no remote servers, no browser extensions.
- **Agents interact through structured protocols.** The primary interface is an [MCP](https://modelcontextprotocol.io) server, giving AI agents native wallet access without custom integrations.
- **Transaction policies are enforced before signing.** A pre-signing policy engine gates every operation, so agents can be granted scoped, auditable access to wallet capabilities.
- **One interface covers all chains.** CAIP-2/CAIP-10 addressing and a unified signing interface abstract away chain-specific details across EVM, Solana, Bitcoin, Cosmos, and Tron.

## Repo Structure

```
├── docs/                        # The specification (8 documents)
│   ├── 01-storage-format.md         # Vault layout, Keystore v3, filesystem permissions
│   ├── 02-chain-agnostic-addressing.md  # CAIP-2/CAIP-10 standards
│   ├── 03-signing-interface.md      # sign, signAndSend, signMessage operations
│   ├── 04-policy-engine.md          # Pre-signing transaction policies
│   ├── 05-key-isolation.md          # HD derivation paths and key separation
│   ├── 06-agent-access-layer.md     # MCP server, native language bindings
│   ├── 07-multi-chain-support.md    # Multi-chain account management
│   └── 08-wallet-lifecycle.md       # Creation, recovery, deletion, lifecycle events
│
├── lws/                         # Rust reference implementation
│   └── crates/
│       ├── lws-core/                # Core types, CAIP parsing, config (zero crypto deps)
│       └── lws-signer/             # Signing, HD derivation, chain-specific implementations
│
└── website/                     # Documentation site (localwalletstandard.org)
```

## Getting Started

Read the spec starting with [`docs/01-storage-format.md`](docs/01-storage-format.md), or browse it at [localwalletstandard.org](https://localwalletstandard.org).

Install the reference implementation:

```bash
curl -fsSL https://openwallet.sh/install.sh | bash
```

Or build from source:

```bash
cd lws
cargo build --workspace --release
cargo test --workspace
```

## CLI

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

## Language Bindings

LWS provides native bindings that call directly into the Rust `lws-lib` crate via FFI — no HTTP server or subprocess required.

### Node.js

```bash
npm install @lws/node
```

```typescript
import { createWallet, listWallets, signMessage } from "@lws/node";

const wallet = createWallet("agent-treasury", "evm", "my-passphrase");
console.log(wallet.address);

const wallets = listWallets();
const result = signMessage("agent-treasury", "evm", "hello", "my-passphrase");
console.log(result.signature);
```

### Python

```bash
pip install lws
```

```python
from lws import create_wallet, list_wallets, sign_message

wallet = create_wallet("agent-treasury", "evm", "my-passphrase")
print(wallet["address"])

wallets = list_wallets()
result = sign_message("agent-treasury", "evm", "hello", "my-passphrase")
print(result["signature"])
```

Both bindings are compiled native modules (NAPI for Node.js, PyO3 for Python) and share the same API surface — wallet management, signing, and mnemonic operations.

## License

MIT
