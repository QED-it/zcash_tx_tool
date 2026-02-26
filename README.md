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
- [Testing Block Data Storage Locally](#testing-block-data-storage-locally)
    - [Docker Volume Mount for Block Data Persistence](#docker-volume-mount-for-block-data-persistence)
- [Connecting to the Public ZSA Testnet](#connecting-to-the-public-ZSA-testnet)
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
git clone -b zsa-integration-demo --single-branch https://github.com/QED-it/zebra.git

# Navigate to the testnet deployment directory
cd zebra/testnet-single-node-deploy

# Build the Zebra Docker image
docker build -t qedit/zebra-regtest-txv6 .

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

# Set up the database
diesel setup

# Get Zcash Params for Sapling (if needed)
./zcutil/fetch-params.sh
```

#### Build and Run a Test Scenario

There are multiple test scenarios provided in the repository, viz.
* `test-orchard-zsa` (The detailed script for the flow is at [test_orchard_zsa.rs](src/commands/test_orchard_zsa.rs).)
* `test-three-party` (The detailed script for the flow is at [test_three_party.rs](src/commands/test_three_party.rs).)
* `test-orchard` (The detailed script for the flow is at [test_orchard.rs](src/commands/test_orchard.rs).)

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
   diesel setup
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

## Testing Block Data Storage Locally

The `tx-tool` includes a block data storage feature that speeds up wallet synchronization and enables chain reorganization detection. You can test this feature locally by running the GitHub Actions workflow with `act`.

### Using `act` to Run GitHub Actions Locally

[`act`](https://github.com/nektos/act) allows you to run GitHub Actions workflows on your local machine:

```bash
# Install act (macOS)
brew install act

# Install act (Linux)
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# Run the block data demo workflow
act workflow_dispatch -W .github/workflows/block-data-test-ci.yaml

# Or run on push event (simulating a push to main)
act push -W .github/workflows/block-data-test-ci.yaml
```

**Note**: The workflow requires significant disk space (~20GB) and may take 15-30 minutes to complete due to Docker image builds. Subsequent runs will be faster, as Docker caches images locally.

### Understanding Block Data Storage Behavior

The block data storage stores:
- **Block hashes**: For chain validation and reorg detection  
- **Transaction data**: To avoid re-downloading blocks

On subsequent runs, the tool:
1. Validates the stored chain matches the node's chain
2. Resumes sync from the last stored block (if valid)
3. Detects and handles chain reorganizations

**Note**: Test commands call `reset()` which clears wallet state but preserves the block data. For full persistence (skipping wallet rescan entirely), ensure wallet state persists between runs.

## Block Data Storage Considerations

When block data storage is enabled, disk usage depends on average block size and the amount of blocks.

There are currently (January 2026) ~3.2M blocks on Zcash mainnet (i.e., the total number of blocks mined since genesis at the time of writing).
Approximate totals for ~3.2M Zcash blocks:

- **Minimal blocks (0.5–1 KB):** ~1.6–3.2 GB
- **Average blocks (50–100 KB):** ~160–322 GB
- **Heavy blocks (300–500 KB):** ~1.0–1.6 TB

**Notes:**
- Regtest / ZSA testnet runs are usually near the minimal range.
- Long-running testnet or mainnet syncs trend toward the average case.
- Disk usage grows over time unless block data is pruned.

### Docker Volume Mount for Block Data Persistence

When running the tx-tool in Docker, you **must** use the `-v` flag to mount a volume for block data persistence. The Docker volume mount cannot be configured from inside the Dockerfile — the host running the command needs to specify it explicitly.

```bash
docker run --network zcash-net \
  -e ZCASH_NODE_ADDRESS=zebra-node \
  -e ZCASH_NODE_PORT=18232 \
  -e ZCASH_NODE_PROTOCOL=http \
  -v wallet-data:/app \
  zcash-tx-tool:local test-orchard-zsa
```

The `-v wallet-data:/app` flag creates a named Docker volume (`wallet-data`) and mounts it at `/app` inside the container. This is where the tx-tool stores its block data (SQLite database and related files).

**Without `-v wallet-data:/app`**, Docker will start the container with an empty directory at `/app`. The tx-tool will still run, but all block data will be ephemeral and lost when the container exits, so subsequent runs won't benefit from the cached block data.

### About the Workflow

The `act` tool runs the GitHub Actions workflow locally, which uses Docker to build and run both the Zebra node and the tx-tool in containers. This approach is similar to the manual Docker setup described in the [Getting Started](#getting-started) section above, where we build Docker images and run them with environment variables and volume mounts. The workflow automates this process and demonstrates block data persistence between multiple runs of the tx-tool.

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
