# Node.js SDK

> Native bindings for Node.js via NAPI. No CLI, no server, no subprocess &mdash; the Rust core runs in-process.

[![npm](https://img.shields.io/npm/v/@local-wallet-standard/node)](https://www.npmjs.com/package/@local-wallet-standard/node)

## Install

```bash
npm install @local-wallet-standard/node
```

The package includes prebuilt native binaries for macOS (arm64, x64) and Linux (x64, arm64). No Rust toolchain required.

## Quick Start

```javascript
import {
  generateMnemonic,
  createWallet,
  listWallets,
  signMessage,
  deleteWallet,
} from "@local-wallet-standard/node";

const mnemonic = generateMnemonic(12);
const wallet = createWallet("my-wallet");
const sig = signMessage("my-wallet", "evm", "hello");
console.log(sig.signature);
```

## API Reference

### Types

```typescript
interface AccountInfo {
  chainId: string;        // CAIP-2 chain ID (e.g. "eip155:1")
  address: string;        // Chain-native address
  derivationPath: string; // BIP-44 path (e.g. "m/44'/60'/0'/0/0")
}

interface WalletInfo {
  id: string;             // UUID v4
  name: string;
  accounts: AccountInfo[];
  createdAt: string;      // ISO 8601
}

interface SignResult {
  signature: string;      // Hex-encoded signature
  recoveryId?: number;    // EVM/Tron recovery ID (v value)
}

interface SendResult {
  txHash: string;         // Transaction hash
}
```

### Mnemonic

#### `generateMnemonic(words?)`

Generate a new BIP-39 mnemonic phrase.

```javascript
const phrase = generateMnemonic(12);  // or 24
// => "goose puzzle decorate much stable beach ..."
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `words` | `number` | `12` | Word count (12 or 24) |

**Returns:** `string`

#### `deriveAddress(mnemonic, chain, index?)`

Derive an address from a mnemonic without creating a wallet.

```javascript
const addr = deriveAddress(mnemonic, "evm");
// => "0xCc1e2c3D077b7c0f5301ef400bDE30d0e23dF1C6"

const solAddr = deriveAddress(mnemonic, "solana");
// => "DzkqyvQrBvLqKSMhCoXoGK65e9PvyWjb6YjS4BqcxN2i"
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `mnemonic` | `string` | &mdash; | BIP-39 mnemonic phrase |
| `chain` | `string` | &mdash; | `"evm"`, `"solana"`, `"bitcoin"`, `"cosmos"`, `"tron"` |
| `index` | `number` | `0` | Account index in derivation path |

**Returns:** `string`

### Wallet Management

#### `createWallet(name, passphrase?, words?, vaultPath?)`

Create a new wallet. Generates a mnemonic and derives addresses for all supported chains.

```javascript
const wallet = createWallet("agent-treasury");
console.log(wallet.accounts);
// => [
//   { chainId: "eip155:1", address: "0x...", derivationPath: "m/44'/60'/0'/0/0" },
//   { chainId: "solana:5eykt4...", address: "7Kz9...", derivationPath: "m/44'/501'/0'/0'" },
//   { chainId: "bip122:000...", address: "bc1q...", derivationPath: "m/84'/0'/0'/0/0" },
//   { chainId: "cosmos:cosmoshub-4", address: "cosmos1...", derivationPath: "m/44'/118'/0'/0/0" },
//   { chainId: "tron:mainnet", address: "TKLm...", derivationPath: "m/44'/195'/0'/0/0" },
// ]
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | &mdash; | Wallet name |
| `passphrase` | `string` | `undefined` | Encryption passphrase |
| `words` | `number` | `12` | Mnemonic word count |
| `vaultPath` | `string` | `~/.lws/wallets` | Custom vault directory |

**Returns:** `WalletInfo`

#### `listWallets(vaultPath?)`

List all wallets in the vault.

```javascript
const wallets = listWallets();
console.log(wallets.length); // => 1
```

**Returns:** `WalletInfo[]`

#### `getWallet(nameOrId, vaultPath?)`

Look up a wallet by name or UUID.

```javascript
const wallet = getWallet("agent-treasury");
```

**Returns:** `WalletInfo`

#### `deleteWallet(nameOrId, vaultPath?)`

Delete a wallet from the vault.

```javascript
deleteWallet("agent-treasury");
```

#### `renameWallet(nameOrId, newName, vaultPath?)`

Rename a wallet.

```javascript
renameWallet("old-name", "new-name");
```

#### `exportWallet(nameOrId, passphrase?, vaultPath?)`

Export a wallet's secret (mnemonic or private key).

```javascript
const mnemonic = exportWallet("agent-treasury");
// => "goose puzzle decorate much ..."
```

**Returns:** `string`

### Import

#### `importWalletMnemonic(name, mnemonic, passphrase?, index?, vaultPath?)`

Import a wallet from a BIP-39 mnemonic.

```javascript
const wallet = importWalletMnemonic("imported", "goose puzzle decorate ...");
```

**Returns:** `WalletInfo`

#### `importWalletPrivateKey(name, privateKeyHex, passphrase?, vaultPath?)`

Import a wallet from a hex-encoded private key.

```javascript
const wallet = importWalletPrivateKey("imported", "4c0883a691...");
```

**Returns:** `WalletInfo`

### Signing

#### `signMessage(wallet, chain, message, passphrase?, encoding?, index?, vaultPath?)`

Sign a message with chain-specific formatting.

```javascript
const result = signMessage("agent-treasury", "evm", "hello world");
console.log(result.signature);  // hex string
console.log(result.recoveryId); // 0 or 1
```

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `wallet` | `string` | &mdash; | Wallet name or ID |
| `chain` | `string` | &mdash; | Chain family |
| `message` | `string` | &mdash; | Message to sign |
| `passphrase` | `string` | `undefined` | Decryption passphrase |
| `encoding` | `string` | `"utf8"` | `"utf8"` or `"hex"` |
| `index` | `number` | `0` | Account index |
| `vaultPath` | `string` | `~/.lws/wallets` | Custom vault directory |

**Returns:** `SignResult`

#### `signTransaction(wallet, chain, txHex, passphrase?, index?, vaultPath?)`

Sign a raw transaction (hex-encoded bytes).

```javascript
const result = signTransaction("agent-treasury", "evm", "02f8...");
console.log(result.signature);
```

**Returns:** `SignResult`

#### `signAndSend(wallet, chain, txHex, passphrase?, index?, rpcUrl?, vaultPath?)`

Sign and broadcast a transaction. Requires an RPC URL.

```javascript
const result = signAndSend(
  "agent-treasury", "evm", "02f8...",
  undefined, undefined, "https://mainnet.infura.io/v3/..."
);
console.log(result.txHash);
```

**Returns:** `SendResult`

## Custom Vault Path

Every function accepts an optional `vaultPath` parameter. When omitted, the default vault at `~/.lws/wallets/` is used. This is useful for testing or running isolated environments:

```javascript
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const tmpVault = mkdtempSync(join(tmpdir(), "lws-test-"));

const wallet = createWallet("test-wallet", undefined, 12, tmpVault);
// ... use wallet ...

rmSync(tmpVault, { recursive: true, force: true });
```
