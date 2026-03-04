"""Tests for lws Python bindings."""

import tempfile
import pytest
import lws

PASSPHRASE = "supersecretpass!"


@pytest.fixture
def vault_dir():
    with tempfile.TemporaryDirectory() as d:
        yield d


def test_generate_mnemonic_12():
    phrase = lws.generate_mnemonic(12)
    assert len(phrase.split()) == 12


def test_generate_mnemonic_24():
    phrase = lws.generate_mnemonic(24)
    assert len(phrase.split()) == 24


def test_derive_address_evm():
    phrase = lws.generate_mnemonic(12)
    address = lws.derive_address(phrase, "evm")
    assert address.startswith("0x")
    assert len(address) == 42


def test_create_and_list_wallets(vault_dir):
    wallet = lws.create_wallet("test-wallet", "evm", PASSPHRASE, vault_path_opt=vault_dir)
    assert wallet["name"] == "test-wallet"
    assert wallet["chain"] == "evm"
    assert wallet["address"].startswith("0x")

    wallets = lws.list_wallets(vault_path_opt=vault_dir)
    assert len(wallets) == 1
    assert wallets[0]["id"] == wallet["id"]


def test_get_wallet(vault_dir):
    wallet = lws.create_wallet("lookup", "evm", PASSPHRASE, vault_path_opt=vault_dir)

    found = lws.get_wallet("lookup", vault_path_opt=vault_dir)
    assert found["id"] == wallet["id"]

    found = lws.get_wallet(wallet["id"], vault_path_opt=vault_dir)
    assert found["name"] == "lookup"


def test_rename_wallet(vault_dir):
    lws.create_wallet("old-name", "evm", PASSPHRASE, vault_path_opt=vault_dir)
    lws.rename_wallet("old-name", "new-name", vault_path_opt=vault_dir)

    found = lws.get_wallet("new-name", vault_path_opt=vault_dir)
    assert found["name"] == "new-name"


def test_export_wallet(vault_dir):
    lws.create_wallet("exportable", "evm", PASSPHRASE, vault_path_opt=vault_dir)
    secret = lws.export_wallet("exportable", PASSPHRASE, vault_path_opt=vault_dir)
    assert len(secret.split()) == 12


def test_delete_wallet(vault_dir):
    wallet = lws.create_wallet("deletable", "evm", PASSPHRASE, vault_path_opt=vault_dir)
    lws.delete_wallet("deletable", vault_path_opt=vault_dir)

    wallets = lws.list_wallets(vault_path_opt=vault_dir)
    assert len(wallets) == 0


def test_import_wallet_mnemonic(vault_dir):
    phrase = lws.generate_mnemonic(12)
    expected_addr = lws.derive_address(phrase, "evm")

    wallet = lws.import_wallet_mnemonic(
        "imported", "evm", phrase, PASSPHRASE, vault_path_opt=vault_dir
    )
    assert wallet["name"] == "imported"
    assert wallet["address"] == expected_addr


def test_sign_transaction(vault_dir):
    lws.create_wallet("signer", "evm", PASSPHRASE, vault_path_opt=vault_dir)

    tx_hex = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    result = lws.sign_transaction(
        "signer", "evm", tx_hex, PASSPHRASE, vault_path_opt=vault_dir
    )
    assert len(result["signature"]) > 0
    assert result["recovery_id"] is not None


def test_sign_message(vault_dir):
    lws.create_wallet("msg-signer", "evm", PASSPHRASE, vault_path_opt=vault_dir)

    result = lws.sign_message(
        "msg-signer", "evm", "hello world", PASSPHRASE, vault_path_opt=vault_dir
    )
    assert len(result["signature"]) > 0
