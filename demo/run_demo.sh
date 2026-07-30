#!/usr/bin/env bash
# Full ZSA wallet demo against a regtest Zebra node.
#
# Two independent wallets (separate seeds, separate note databases) talk to
# the same node:
#   • the ISSUER ("QEDIT Corp") issues, transfers, re-issues and finalizes an asset
#   • ALICE receives it, labels it, spends and burns it
#
# Prerequisites:
#   • a ZSA-enabled Zebra regtest node listening on 127.0.0.1:18232
#     (see README: QED-it/zebra branch zsa-integration-demo)
#   • the wallet built: cargo build --release
#   • Sapling params fetched: ./zcutil/fetch-params.sh
#
# Run from the repository root:  ./demo/run_demo.sh
# For a pristine run, restart the Zebra node (its regtest state is ephemeral)
# and delete demo/*.sqlite.

set -euo pipefail

BIN=./target/release/zcash_tx_tool
ISSUER="$BIN --config demo/issuer.toml"
ALICE="$BIN --config demo/alice.toml"

# Unique asset name per run so the script can be re-run against a used chain.
ASSET="QEDIT-DEMO-$RANDOM"

step() {
    echo
    echo "══════════════════════════════════════════════════════════════════"
    echo "  $1"
    echo "══════════════════════════════════════════════════════════════════"
}

step "1. Wallet status and addresses"
$ISSUER status
$ALICE addresses
ALICE_UA=$($ALICE addresses | awk '/unified/{print $3; exit}')

step "2. Issuer creates a new shielded asset: 1,000,000 units of $ASSET"
$ISSUER issue "$ASSET" 1000000

step "3. Issuer transfers 250,000 units to Alice's unified address"
$ISSUER transfer "$ASSET" 250000 --to "$ALICE_UA"

step "4. Alice syncs — the asset appears (known only by its on-chain id)"
$ALICE balance

step "5. Alice labels the asset for her own bookkeeping"
# Pick the AssetBase out of the listing by its shape: the table ends with a
# rule and a blank line, so reading the last line's last field yields "".
BASE=$($ALICE assets | grep -oE '[0-9a-f]{64}' | tail -1)
if [ -z "$BASE" ]; then
    echo "ERROR: could not find an AssetBase in Alice's asset listing" >&2
    exit 1
fi
$ALICE assets --label "$BASE" --name "QEDIT Demo Token"
$ALICE assets

step "6. Alice burns 50,000 units (provable supply reduction)"
$ALICE burn "QEDIT Demo Token" 50000

step "7. Issuer re-issues 500,000 more units (supply is still open)"
$ISSUER issue "$ASSET" 500000

step "8. Issuer finalizes the asset — issuance is closed forever"
$ISSUER finalize "$ASSET"

step "9. Issuance after finalization is rejected"
if $ISSUER issue "$ASSET" 1; then
    echo "ERROR: issuance after finalization should have failed" >&2
    exit 1
else
    echo "(rejected as expected)"
fi

step "10. Native ZEC works too: shield a coinbase and pay Alice 1 ZEC"
$ISSUER shield
$ISSUER transfer zec 100000000 --to "$ALICE_UA"

step "11. Final state: notes and balances of both wallets"
$ISSUER notes
$ALICE balance
$ALICE notes

echo
echo "Demo complete."
