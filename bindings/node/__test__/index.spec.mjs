import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// The native module is built by `napi build` into the project root.
import {
  generateMnemonic,
  deriveAddress,
  createWallet,
  listWallets,
  getWallet,
  deleteWallet,
  exportWallet,
  renameWallet,
  importWalletMnemonic,
  signTransaction,
  signMessage,
} from '../index.js';

const PASSPHRASE = 'supersecretpass!';

describe('@lws/node', () => {
  let vaultDir;

  before(() => {
    vaultDir = mkdtempSync(join(tmpdir(), 'lws-node-test-'));
  });

  after(() => {
    rmSync(vaultDir, { recursive: true, force: true });
  });

  it('generates a 12-word mnemonic', () => {
    const phrase = generateMnemonic(12);
    assert.equal(phrase.split(' ').length, 12);
  });

  it('generates a 24-word mnemonic', () => {
    const phrase = generateMnemonic(24);
    assert.equal(phrase.split(' ').length, 24);
  });

  it('derives an EVM address from a mnemonic', () => {
    const phrase = generateMnemonic(12);
    const address = deriveAddress(phrase, 'evm');
    assert.ok(address.startsWith('0x'));
    assert.equal(address.length, 42);
  });

  it('creates, lists, gets, renames, exports, and deletes a wallet', () => {
    const wallet = createWallet('test-wallet', 'evm', PASSPHRASE, 12, vaultDir);
    assert.equal(wallet.name, 'test-wallet');
    assert.equal(wallet.chain, 'evm');
    assert.ok(wallet.address.startsWith('0x'));

    // List
    const wallets = listWallets(vaultDir);
    assert.equal(wallets.length, 1);
    assert.equal(wallets[0].id, wallet.id);

    // Get by name
    const found = getWallet('test-wallet', vaultDir);
    assert.equal(found.id, wallet.id);

    // Rename
    renameWallet('test-wallet', 'renamed-wallet', vaultDir);
    const renamed = getWallet('renamed-wallet', vaultDir);
    assert.equal(renamed.id, wallet.id);

    // Export
    const secret = exportWallet('renamed-wallet', PASSPHRASE, vaultDir);
    assert.equal(secret.split(' ').length, 12);

    // Delete
    deleteWallet('renamed-wallet', vaultDir);
    const afterDelete = listWallets(vaultDir);
    assert.equal(afterDelete.length, 0);
  });

  it('imports a wallet from mnemonic', () => {
    const phrase = generateMnemonic(12);
    const expectedAddr = deriveAddress(phrase, 'evm');

    const wallet = importWalletMnemonic(
      'imported',
      'evm',
      phrase,
      PASSPHRASE,
      undefined,
      vaultDir,
    );

    assert.equal(wallet.name, 'imported');
    assert.equal(wallet.address, expectedAddr);

    // Cleanup
    deleteWallet('imported', vaultDir);
  });

  it('signs a transaction', () => {
    createWallet('signer', 'evm', PASSPHRASE, 12, vaultDir);

    const txHex = 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef';
    const result = signTransaction(
      'signer',
      'evm',
      txHex,
      PASSPHRASE,
      undefined,
      vaultDir,
    );

    assert.ok(result.signature.length > 0);
    assert.ok(result.recoveryId !== undefined);

    deleteWallet('signer', vaultDir);
  });

  it('signs a message', () => {
    createWallet('msg-signer', 'evm', PASSPHRASE, 12, vaultDir);

    const result = signMessage(
      'msg-signer',
      'evm',
      'hello world',
      PASSPHRASE,
      undefined,
      undefined,
      vaultDir,
    );

    assert.ok(result.signature.length > 0);

    deleteWallet('msg-signer', vaultDir);
  });
});
