# Zcash tx-tool — a ZSA wallet CLI

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

The **Zcash tx-tool** is a command-line wallet for **Zcash Shielded Assets (ZSA)**. It manages shielded notes, and can **issue**, **transfer**, **burn** and **finalize** Orchard-ZSA assets — as well as handle native ZEC — against a ZSA-enabled node (e.g., the QED-it fork of Zebra). It supports transaction versions V5 and V6, including the Orchard ZSA functionality (ZIP 226/227/230).

This repository includes a simple Zebra Docker image that incorporates the OrchardZSA version of Zebra and runs in regtest mode.

WARNING: This wallet is alpha software intended for regtest and testnet experimentation with the in-development ZSA protocol. Do not use it to hold real funds.

## Table of Contents

- [Features](#features)
- [Core Components](#core-components)
- [Prerequisites](#prerequisites)
- [Getting Started](#getting-started)
    - [1. Build and Run the Zebra Docker Image](#1-build-and-run-the-zebra-docker-image)
    - [2. Set Up and Run the Zcash tx-tool](#2-set-up-and-run-the-zcash-transaction-tool)
- [Wallet Usage](#wallet-usage)
    - [Wallet Commands](#wallet-commands)
    - [Asset References and Recipients](#asset-references-and-recipients)
    - [Multi-Wallet Demo](#multi-wallet-demo)
    - [Notes on Fees and Block Production](#notes-on-fees-and-block-production)
- [Configuration](#configuration)
- [Build Instructions](#build-instructions)
- [Test Scenarios](#test-scenarios)
    - [Orchard-ZSA Two Party Scenario](#orchard-zsa-two-party-scenario)
    - [Orchard-ZSA Three Party Scenario](#orchard-zsa-three-party-scenario)
    - [Creating your own scenario](#creating-your-own-scenario)
- [Block Data Storage](#block-data-storage)
- [Block Data Storage Considerations](#block-data-storage-considerations)
- [Running the tx-tool in Docker](#running-the-tx-tool-in-docker)
- [Exporting OrchardZSA Test Blocks](#exporting-orchardzsa-test-blocks)
- [Connecting to the Public ZSA Testnet](#connecting-to-the-public-zsa-testnet)
- [License](#license)
- [Acknowledgements](#acknowledgements)

## Features

- **Note Management**: Tracks, persists and selects shielded Orchard notes across accounts; survives restarts and full rescans.
- **ZSA Issuance / Transfer / Burn / Finalize**: The complete Orchard-ZSA asset lifecycle (ZIP 227 issuance, ZIP 226 transfers and burns), from the command line.
- **Asset Registry**: Local registry of assets — own issued assets keep their metadata; received assets are discovered automatically and can be labelled.
- **Native ZEC**: Shield regtest coinbase rewards and transfer ZEC alongside custom assets.
- **Unified Addresses**: Displays and accepts Orchard receivers as unified addresses.
- **Transaction Submission**: Self-mines blocks on regtest via `getblocktemplate`/`submitblock` (mempool submission available where node policy allows).
- **Version Compatibility**: Supports transaction versions V5 and V6.

## Supported systems
- Tested on Ubuntu 22.04 LTS but should work on any Linux distribution that support the Prerequisites.

## Status
- **Alpha** - Everything, including APIs and data structures, is subject to breaking changes. Feature set is incomplete.

## Core Components

1. **[librustzcash](https://github.com/zcash/librustzcash)**: Used for transaction creation and serialization. This version includes slight modifications for additional functionality.
2. **[Diesel ORM Framework](https://diesel.rs/)**: A safe and extensible ORM and query builder for Rust.
3. **[Abscissa Framework](https://github.com/iqlusioninc/abscissa)**: A microframework for building Rust applications.

## Prerequisites

- **Docker**: [Install Docker](https://www.docker.com/get-started)
- **Rust & Cargo**: [Install Rust and Cargo](https://www.rust-lang.org/tools/install)
- **Diesel CLI**: Installed via Cargo.
- **Linux Dev tools**:
```bash
sudo apt update

sudo apt install pkg-config libssl-dev libsqlite3-dev 
```

## Getting Started

### 1. Build and Run the Zebra Docker Image

Open a terminal and execute the following commands:

```bash
# Clone the zebra repository with the ZSA integration branch
git clone -b zsa-integration-demo --single-branch --depth=1 https://github.com/QED-it/zebra.git

# Navigate to the Zebra directory
cd zebra

# Build the Zebra Docker image
docker build -t qedit/zebra-regtest-txv6 -f testnet-single-node-deploy/dockerfile .

# Run the Zebra Docker container
docker run -p 18232:18232 qedit/zebra-regtest-txv6
```

For more details on how the Docker image is created and synchronized, refer to the [Dockerfile](https://github.com/QED-it/zebra/blob/zsa-integration-demo/testnet-single-node-deploy/dockerfile) in the zebra repository.

### 2. Set Up and Run the Zcash tx-tool

In a separate terminal window, perform the following steps:

#### One-Time Setup

Install Diesel CLI and set up the database and get Zcash Params for Sapling:

```bash
# Install Diesel CLI with SQLite support
cargo install diesel_cli --no-default-features --features sqlite

# Set up the database (Diesel CLI requires DATABASE_URL)
DATABASE_URL=walletdb.sqlite diesel setup

# Get Zcash Params for Sapling (if needed)
./zcutil/fetch-params.sh
```

The application uses the same default as above: if `DATABASE_URL` is not set at runtime, it connects to `walletdb.sqlite`.

#### Build and Run a Test Scenario

There are multiple test scenarios provided in the repository, viz.
* `test-orchard-zsa` (The detailed script for the flow is at [test_orchard_zsa.rs](src/commands/test_orchard_zsa.rs).)
* `test-three-party` (The detailed script for the flow is at [test_three_party.rs](src/commands/test_three_party.rs).)
* `test-orchard` (The detailed script for the flow is at [test_orchard.rs](src/commands/test_orchard.rs).)
* `test-issue-one` (The detailed script for the flow is at [test_issue_one.rs](src/commands/test_issue_one.rs).)

Build and run the test case of your choice using the Zcash Transaction Tool, by replacing `<test-case>` in the command below with either of the test scenarios listed above:

```bash
# Build and run with ZSA feature enabled
cargo run --release --package zcash_tx_tool --bin zcash_tx_tool <test-case>
```

For example, to run the `test-orchard-zsa` scenario, use:

```bash
cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-orchard-zsa
```

**Note**: To re-run the test scenario (or to run a different scenario), reset the Zebra node by stopping and restarting the Zebra Docker container.

## Wallet Usage

Once the node is running and the tool is built (see [Getting Started](#getting-started)), the wallet is operated through subcommands:

```bash
# See node connection and wallet state
zcash_tx_tool status

# Show your shielded receiving addresses (unified + raw hex)
zcash_tx_tool addresses

# Issue 1,000,000 units of a new shielded asset to account 0
zcash_tx_tool issue "MY-TOKEN" 1000000

# Send 250,000 units to another wallet's unified address
zcash_tx_tool transfer "MY-TOKEN" 250000 --to uregtest1...

# Burn 50,000 units (provably reduce supply)
zcash_tx_tool burn "MY-TOKEN" 50000

# Permanently close the asset's supply
zcash_tx_tool finalize "MY-TOKEN"

# Balances, notes and known assets
zcash_tx_tool balance
zcash_tx_tool notes
zcash_tx_tool assets
```

(When running from the repository, prefix with `cargo run --release --` or use `./target/release/zcash_tx_tool`.)

### Wallet Commands

| Command | Description |
| --- | --- |
| `status` | Node connection, chain tip, wallet sync height, note/asset counts |
| `sync` | Scan new blocks for the wallet's notes and balances |
| `addresses [--accounts N]` | Unified + raw Orchard receiving addresses per account |
| `balance [--no-sync]` | Balance table: every known asset × every account |
| `notes [--all]` | The wallet's notes (unspent by default) |
| `assets [--label <base> --name <text>]` | List known ZSA assets, or label a discovered one |
| `issue <desc> <amount> [--to R]` | Issue a new asset, or more of an existing one |
| `transfer <asset> <amount> --to R [--from-account N]` | Send asset units (or `zec`) shielded |
| `burn <asset> <amount> [--from-account N]` | Provably destroy asset units |
| `finalize <asset>` | Permanently stop issuance of an own asset |
| `mine [--blocks N]` | Produce regtest block(s), including mempool txs |
| `shield [--to-account N]` | Mine + shield a coinbase reward (regtest ZEC faucet) |
| `clean` | Reset all local wallet state (rescans on next sync) |

All state-changing commands accept `--mempool` to submit via `sendrawtransaction` instead of self-mining (see [fees](#notes-on-fees-and-block-production)).

### Asset References and Recipients

- **Assets** can be referenced by their description string (for assets you issued or labelled), by a unique prefix of the hex AssetBase, or by the full 64-char hex. `zec`/`native` selects the native asset.
- **Recipients** can be `account:<n>` (an own account), a unified address (`uregtest1…`), or an 86-char raw Orchard address hex.

### Multi-Wallet Demo

A complete two-party demo — issuer and receiver wallets with separate seeds and note databases against one regtest node — is provided in [`demo/`](./demo):

```bash
./demo/run_demo.sh
```

It walks through the full asset lifecycle: issue → transfer → discovery + labelling on the receiving side → burn → re-issue → finalize → post-finalization rejection, plus native ZEC shielding and payment. The wallet configs (`demo/issuer.toml`, `demo/alice.toml`) show how to run several wallets side by side.

### Notes on Fees and Block Production

On regtest the wallet acts as the block producer: transactions are placed directly into a block the wallet assembles and submits (`getblocktemplate` → `submitblock`), which is also how the CI test scenarios run. Transactions are currently built with a zero fee, which consensus permits — but node *mempool policy* (ZIP-317) rejects unpaid actions, so `--mempool` submission requires a node with relaxed policy. ZIP-317 fee support is on the roadmap.

## Configuration

You can specify the path to the configuration file using the `--config` flag when running the application. The default configuration file name is `config.toml`.

An example configuration file with default values is provided in [`regtest_config.toml`](./regtest-config.toml).

Wallet-specific settings in the `[wallet]` section: `seed_phrase` (BIP-39 wallet identity), `miner_seed_phrase` (must match the node's coinbase address for `shield`), `num_accounts` (accounts scanned during sync, default 3), and `db_path` (per-wallet SQLite database, overriding `DATABASE_URL`).

## Build Instructions

To set up the Diesel database:

1. **Install Diesel CLI**:

   ```bash
   cargo install diesel_cli --no-default-features --features sqlite
   ```

2. **Set Up the Database**:

   ```bash
   DATABASE_URL=walletdb.sqlite diesel setup
   ```

To build the application:

```bash
# Debug build
cargo build

# Release build (recommended for performance)
cargo build --release
```

To test ZSA functionality with the tool, enable the corresponding feature flag:

```bash
cargo build --release
```

## Test Scenarios

We describe here 

### Orchard-ZSA Two Party Scenario

This test scenario ([src/commands/test_orchard_zsa.rs](src/commands/test_orchard_zsa.rs)) is a two-party setting which performs the following steps:

1. **Issue an Asset**: Create and issue a new ZSA.
2. **Transfer the Asset**: Send the issued asset to another account.
3. **Burn the Asset (Twice)**: Burn the asset in two separate transactions.

To run the test scenario:

```bash
cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-orchard-zsa
```

### Orchard-ZSA Three Party Scenario

This test scenario ([src/commands/test_three_party.rs](src/commands/test_three_party.rs)) is a three-party setting which performs the following steps:

1. **Issue an Asset**: Create and issue a new ZSA.
2. **Transfer the Asset (Twice)**: Send the issued ZSA to another account, and then from that account to a third account.
3. **Burn the Asset**: The third account burns the ZSA.

To run the test scenario:

```bash
cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-three-party
```

### Issue One Asset Scenario

This test scenario ([src/commands/test_issue_one.rs](src/commands/test_issue_one.rs)) is a minimal test that performs only the asset issuance step:

1. **Issue an Asset**: Create and issue a single ZSA asset (1 unit).

This simplified scenario is useful for quick testing of the asset issuance functionality without the complexity of transfers and burns.

To run the test scenario:

```bash
cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-issue-one
```

### Creating your own scenario
It is also possible to construct your own scenario in a manner similar to these. 
To do so, copy one of the test scenario files to a new file in the same location and make the changes to fit your setting.

To allow this new file to be run, make the following changes to [commands.rs](src/commands.rs):
* Add the module corresponding to your new file to the start of [commands.rs](src/commands.rs).
* Add an analogous new variant to the `AppCmd` enum.

You should then be able to run your scenario via (assuming `test-scenario` is the name of your scenario):
```bash
cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-scenario
```

## Block Data Storage

The `tx-tool` records block hashes locally so later runs can validate the stored chain head and detect chain reorganizations.

The block data storage stores:
- **Block hashes**: For chain validation and reorg detection
- **Wallet tree state**: The note commitment tree and last synced block

On subsequent runs, the tool:
1. Validates the stored chain matches the node's chain
2. Resumes sync from the persisted wallet head when wallet state is consistent with `block_data`
3. Uses preserved block hashes to validate rescans after `reset()`
4. On any chain reorganization (or wallet/block-data inconsistency), wipes all persisted state (`block_data`, `wallet_state`, notes, commitment tree) and resyncs from scratch — there is no per-block rollback or partial rewind

**Note**: `Wallet::reset` (and the `clean` subcommand) wipes everything: `block_data`, `wallet_state`, notes, and the in-memory tree. Subsequent runs auto-load any persisted `wallet_state` row and resume sync from `wallet_head + 1`, with no full re-sync.

## Block Data Storage Considerations

The tx-tool stores two pieces of state on disk:

- **`block_data` table:** one row per synced block (`height` + hex-encoded 32-byte `hash`). Each row is ~100 bytes including SQLite overhead, **independent of the block's transaction size**. Storage scales linearly with block count.
- **`wallet_state` table:** a single row holding the serialized commitment tree, last synced height, and last synced hash. Size scales with `O(N * log(T / N))`, where `N` is the number of wallet notes and `T` is the total chain commitments.

There are currently (January 2026) ~3.2M blocks on Zcash mainnet. Approximate totals at that scale:

- **`block_data`:** ~100 bytes per block × 3.2M ≈ **~300 MB** on mainnet (< 1 MB on regtest / ZSA testnet).
- **`wallet_state`:** ~2 MB for 1K notes / 5M commitments on mainnet (a few KB on regtest / ZSA testnet).

**Notes:**
- `block_data` storage is bounded by block count, not block size, so heavy mainnet blocks don't make it any larger.
- `wallet_state` is rewritten in-place on each sync step, so it does not grow with sync time, only with wallet activity.
- Disk usage grows over time on mainnet unless old `block_data` rows are pruned (not implemented yet).

## Running the tx-tool in Docker

The tx-tool is normally built and run natively, as described above. A Docker workflow is also supported for CI and self-contained deployments. See [`docs/tx_tool_docker_setup.md`](docs/tx_tool_docker_setup.md) for the build, persistence-volume layout, and a host-network example, plus a pointer to the multi-container recipe in `.github/workflows/zebra-test-ci.yaml`.

## Exporting OrchardZSA Test Blocks

Every CI run of `test-orchard-zsa` uploads the full hex of each submitted block as the `orchard-zsa-blocks` artifact (one block per line). Download it from a run with the [GitHub CLI](https://cli.github.com) (use `gh run list` to find run IDs):

```bash
gh run download <run-id> -n orchard-zsa-blocks
```

This writes the blocks to `orchard-zsa-blocks/zsa-blocks.txt`.

To produce the same file locally, set `ZSA_DUMP_BLOCKS` when running the scenario — the tool then prints each submitted block as a `ZSA_BLOCK_HEX <hex>` line on stdout (normal logs stay on stderr):

```bash
ZSA_DUMP_BLOCKS=1 cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-orchard-zsa \
  | awk '/^ZSA_BLOCK_HEX /{print $2}' > zsa-blocks.txt
```

## Connecting to the Public ZSA Testnet

For instructions on connecting to the public ZSA testnet (including available endpoints, environment variables, and example usage), see the [Running tx‑tool](https://github.com/QED-it/zcash_tx_tool/wiki/Running-tx%E2%80%90tool) wiki page.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgements

- **[Zcash](https://z.cash/)**: The privacy-protecting digital currency.
- **[Zebra](https://github.com/ZcashFoundation/zebra)**: An independent Zcash node implementation.
- **[librustzcash](https://github.com/zcash/librustzcash)**: The Rust library underpinning Zcash.
- **[Diesel ORM Framework](https://diesel.rs/)**: For database interactions.
- **[Abscissa Framework](https://github.com/iqlusioninc/abscissa)**: For application structure.

---

Feel free to contribute to this project by opening issues or submitting pull requests.
