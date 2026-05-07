# Zcash tx-tool

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

The **Zcash tx-tool** is designed to create and send Zcash transactions to a node (e.g., Zebra). It currently supports transaction versions V5 and V6, including the Orchard ZSA (Zcash Shielded Assets) functionality.

This repository includes a simple Zebra Docker image that incorporates the OrchardZSA version of Zebra and runs in regtest mode.

WARNING: This tool is not a wallet and should not be used as a wallet. This tool is in the early stages of development and should not be used in production environments.

## Table of Contents

- [Features](#features)
- [Core Components](#core-components)
- [Prerequisites](#prerequisites)
- [Getting Started](#getting-started)
    - [1. Build and Run the Zebra Docker Image](#1-build-and-run-the-zebra-docker-image)
    - [2. Set Up and Run the Zcash tx-tool](#2-set-up-and-run-the-zcash-transaction-tool)
- [Configuration](#configuration)
- [Build Instructions](#build-instructions)
- [Test Scenarios](#test-scenarios)
    - [Orchard-ZSA Two Party Scenario](#orchard-zsa-two-party-scenario)
    - [Orchard-ZSA Three Party Scenario](#orchard-zsa-three-party-scenario)
    - [Creating your own scenario](#creating-your-own-scenario)
- [Block Data Storage](#block-data-storage)
- [Block Data Storage Considerations](#block-data-storage-considerations)
    - [Docker Volume Mount for Block Data Persistence](#docker-volume-mount-for-block-data-persistence)
- [Connecting to the Public ZSA Testnet](#connecting-to-the-public-zsa-testnet)
- [License](#license)
- [Acknowledgements](#acknowledgements)

## Features

- **Transaction Creation**: Craft custom Zcash transactions.
- **Transaction Submission**: Send transactions to a Zcash node.
- **ZSA Support**: Work with Zcash Shielded Assets (Orchard ZSA).
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

## Configuration

You can specify the path to the configuration file using the `--config` flag when running the application. The default configuration file name is `config.toml`.

An example configuration file with default values is provided in [`regtest_config.toml`](./regtest-config.toml).

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

**Crash safety**: each block's three on-disk writes — `block_data` insert, per-tx `notes` inserts, and the `wallet_state` (commitment tree) save — are wrapped in a single SQLite transaction inside `User::process_block`. On any error or panic mid-block the transaction rolls back and the in-memory commitment tree is restored from a snapshot taken on entry. Restarting after a crash sees either the pre-block state or the fully-committed post-block state — never a partial mix.

**Note**: Test commands call `reset()`, which clears wallet notes/tree state but preserves `block_data`. Use `clean`/`reset_full()` when you need to clear both wallet state and stored block hashes. For full persistence that skips wallet rescans entirely, run without calling `reset()` so `wallet_state` can be loaded on startup.

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

### Docker Volume Mount for Block Data Persistence

The container's runtime working directory is `/data` (a directory dedicated to runtime state, separate from the build tree at `/app`). To preserve the SQLite database across container runs, mount a named volume there:

```bash
docker run --network zcash-net \
  -e ZCASH_NODE_ADDRESS=zebra-node \
  -e ZCASH_NODE_PORT=18232 \
  -e ZCASH_NODE_PROTOCOL=http \
  -v wallet-data:/data \
  zcash-tx-tool:local test-orchard-zsa
```

The `-v wallet-data:/data` flag creates a named Docker volume (`wallet-data`) and mounts it at `/data`. The tx-tool writes `walletdb.sqlite` (and any other runtime files) there.

**Without `-v wallet-data:/data`**, the database is written into a writable layer that's discarded when the container is removed — block data and wallet state will not survive between runs.

The mount targets `/data`, not `/app`, deliberately: mounting at `/app` would shadow the binary and source tree on subsequent runs, causing the container to execute a stale binary after image rebuilds.

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
