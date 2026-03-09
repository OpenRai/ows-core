# Python SDK

> Native bindings for Python via PyO3. No CLI, no server, no subprocess &mdash; the Rust core runs in-process.

[![PyPI](https://img.shields.io/pypi/v/local-wallet-standard)](https://pypi.org/project/local-wallet-standard/)

## Install

```bash
pip install local-wallet-standard
```

Prebuilt wheels are available for macOS (arm64, x64) and Linux (x64, arm64) on Python 3.9&ndash;3.13.

## Quick Start

```python
from local_wallet_standard import (
    generate_mnemonic,
    create_wallet,
    list_wallets,
    sign_message,
    delete_wallet,
)

mnemonic = generate_mnemonic(12)
wallet = create_wallet("my-wallet")
sig = sign_message("my-wallet", "evm", "hello")
print(sig["signature"])
```

## API Reference

### Return Types

All functions return Python dicts. Wallet functions return:

```python
# WalletInfo
{
    "id": "3198bc9c-...",             # UUID v4
    "name": "my-wallet",
    "created_at": "2026-03-09T...",   # ISO 8601
    "accounts": [
        {
            "chain_id": "eip155:1",
            "address": "0xab16...",
            "derivation_path": "m/44'/60'/0'/0/0",
        },
        # ... one per supported chain
    ],
}

# SignResult
{
    "signature": "bea6b4ee...",       # Hex-encoded
    "recovery_id": 0,                 # EVM/Tron only (None for others)
}

# SendResult
{
    "tx_hash": "0xabc...",
}
```

### Mnemonic

#### `generate_mnemonic(words=12)`

Generate a new BIP-39 mnemonic phrase.

```python
phrase = generate_mnemonic(12)  # or 24
# => "goose puzzle decorate much stable beach ..."
```

#### `derive_address(mnemonic, chain, index=0)`

Derive an address from a mnemonic without creating a wallet.

```python
addr = derive_address(mnemonic, "evm")
# => "0xCc1e2c3D077b7c0f5301ef400bDE30d0e23dF1C6"

sol_addr = derive_address(mnemonic, "solana")
# => "DzkqyvQrBvLqKSMhCoXoGK65e9PvyWjb6YjS4BqcxN2i"
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `mnemonic` | `str` | &mdash; | BIP-39 mnemonic phrase |
| `chain` | `str` | &mdash; | `"evm"`, `"solana"`, `"bitcoin"`, `"cosmos"`, `"tron"` |
| `index` | `int` | `0` | Account index in derivation path |

### Wallet Management

#### `create_wallet(name, passphrase=None, words=12, vault_path=None)`

Create a new wallet. Derives addresses for all supported chains.

```python
wallet = create_wallet("agent-treasury")
for acct in wallet["accounts"]:
    print(f"{acct['chain_id']}: {acct['address']}")
```

#### `list_wallets(vault_path=None)`

List all wallets in the vault.

```python
wallets = list_wallets()
print(len(wallets))
```

#### `get_wallet(name_or_id, vault_path=None)`

Look up a wallet by name or UUID.

```python
wallet = get_wallet("agent-treasury")
```

#### `delete_wallet(name_or_id, vault_path=None)`

Delete a wallet from the vault.

```python
delete_wallet("agent-treasury")
```

#### `rename_wallet(name_or_id, new_name, vault_path=None)`

Rename a wallet.

```python
rename_wallet("old-name", "new-name")
```

#### `export_wallet(name_or_id, passphrase=None, vault_path=None)`

Export a wallet's secret (mnemonic or private key).

```python
mnemonic = export_wallet("agent-treasury")
```

### Import

#### `import_wallet_mnemonic(name, mnemonic, passphrase=None, index=None, vault_path=None)`

Import a wallet from a BIP-39 mnemonic.

```python
wallet = import_wallet_mnemonic("imported", "goose puzzle decorate ...")
```

#### `import_wallet_private_key(name, private_key_hex, passphrase=None, vault_path=None)`

Import a wallet from a hex-encoded private key.

```python
wallet = import_wallet_private_key("imported", "4c0883a691...")
```

### Signing

#### `sign_message(wallet, chain, message, passphrase=None, encoding=None, index=None, vault_path=None)`

Sign a message with chain-specific formatting.

```python
result = sign_message("agent-treasury", "evm", "hello world")
print(result["signature"])   # hex string
print(result["recovery_id"]) # 0 or 1
```

#### `sign_transaction(wallet, chain, tx_hex, passphrase=None, index=None, vault_path=None)`

Sign a raw transaction (hex-encoded bytes).

```python
result = sign_transaction("agent-treasury", "evm", "02f8...")
print(result["signature"])
```

#### `sign_and_send(wallet, chain, tx_hex, passphrase=None, index=None, rpc_url=None, vault_path=None)`

Sign and broadcast a transaction.

```python
result = sign_and_send(
    "agent-treasury", "evm", "02f8...",
    rpc_url="https://mainnet.infura.io/v3/..."
)
print(result["tx_hash"])
```

## Custom Vault Path

Every function accepts an optional `vault_path` parameter for testing or isolation:

```python
import tempfile
import shutil

vault = tempfile.mkdtemp(prefix="lws-test-")
try:
    wallet = create_wallet("test", vault_path=vault)
    # ... use wallet ...
finally:
    shutil.rmtree(vault)
```
