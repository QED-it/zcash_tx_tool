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
diesel setup # Investigate this for the table generations (migrations)

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
act workflow_dispatch -W .github/workflows/cache-demo-ci.yaml

# Or run on push event (simulating a push to main)
act push -W .github/workflows/cache-demo-ci.yaml
```

**Note**: The workflow requires significant disk space (~20GB) and may take 15-30 minutes to complete due to Docker image builds.

### Understanding Block Data Storage Behavior

The block data storage stores:
- **Block hashes**: For chain validation and reorg detection  
- **Transaction data**: To avoid re-downloading blocks

On subsequent runs, the tool:
1. Validates the stored chain matches the node's chain
2. Resumes sync from the last stored block (if valid)
3. Detects and handles chain reorganizations

**Note**: Test commands call `reset()` which clears wallet state but preserves the block data. For full persistence (skipping wallet rescan entirely), ensure wallet state persists between runs.

### About the Workflow

The `act` tool runs the GitHub Actions workflow locally, which uses Docker to build and run both the Zebra node and the tx-tool in containers. This approach is similar to the manual Docker setup described in the [Getting Started](#getting-started) section above, where we build Docker images and run them with environment variables and volume mounts. The workflow automates this process and demonstrates block data persistence between multiple runs of the tx-tool.

## Connecting to the Public ZSA Testnet

We’ve made it easy to test Zcash Shielded Assets (ZSA) functionality by connecting directly to our public Zebra node, hosted by QEDIT on AWS.

### Public Testnet Details

- **TLS-enabled URL**: [https://dev.zebra.zsa-test.net](https://dev.zebra.zsa-test.net)

You can run the `zcash_tx_tool` against our testnet by setting the appropriate environment variables before running your test scenarios.

### Usage with Native Rust Commands

Set the following environment variables before running your test scenarios:

```bash
export ZCASH_NODE_ADDRESS=dev.zebra.zsa-test.net
export ZCASH_NODE_PORT=443
export ZCASH_NODE_PROTOCOL=https
```

Then run your test scenario as usual:

```bash
cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-orchard-zsa
```

Example logs:

```
Using NetworkConfig: node_address = dev.zebra.zsa-test.net ; node_port = 443 ; protocol = https
2025-05-08T08:07:23.864492Z  INFO zcash_tx_tool::components::transactions: Starting sync from height 1
...
2025-05-08T08:08:58.961625Z  INFO zcash_tx_tool::commands::test_balances: === Balances after burning ===
2025-05-08T08:08:58.961634Z  INFO zcash_tx_tool::commands::test_balances: Account 0 balance: 990
2025-05-08T08:08:58.961638Z  INFO zcash_tx_tool::commands::test_balances: Account 1 balance: 1
```

This demonstrates the full ZSA lifecycle and verifies testnet functionality.

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

[//]: # ()
[//]: # (## Docker-based demo)

[//]: # ()
[//]: # (You can also run the tests using docker. To do that you'll need first to build the docker image)

[//]: # ()
[//]: # (```bash)

[//]: # (docker build -t zcash_tx_tool -f Dockerfile .)

[//]: # (```)

[//]: # ()
[//]: # (And after that run the image itself.)

[//]: # (The default connection parameters are set to connect to the zebra-node running on the machine itself &#40;127.0.0.1&#41;)

[//]: # (If you ran the node in a docker container with the command above, you named that container "zebra-node", so you should use that as the ZCASH_NODE_ADDRESS.)

[//]: # (If the node is running on the ECS server, you can connect to it by setting the ZCASH_NODE_ADDRESS=<Domain>.)

[//]: # ()
[//]: # (First, make sure you created the network:)

[//]: # (```bash)

[//]: # (docker network create zcash-network)

[//]: # (```)

[//]: # (And started the node with the network argument, like this)

[//]: # (```bash)

[//]: # (docker run --name zebra-node --network zcash-network -p 18232:18232 qedit/zebra-regtest-txv6)

[//]: # (```)

[//]: # ()
[//]: # (Here are the 3 options &#40;No parameters will default to the first configuration&#41;)

[//]: # ()
[//]: # (```bash)

[//]: # (docker run -it --network zcash-network -e ZCASH_NODE_ADDRESS=127.0.0.1 -e ZCASH_NODE_PORT=18232 -e ZCASH_NODE_PROTOCOL=http zcash_tx_tool)

[//]: # (docker run -it --network zcash-network -e ZCASH_NODE_ADDRESS=zebra-node -e ZCASH_NODE_PORT=18232 -e ZCASH_NODE_PROTOCOL=http zcash_tx_tool)

[//]: # (docker run -it --network zcash-network -e ZCASH_NODE_ADDRESS=<Domain> -e ZCASH_NODE_PORT=18232 -e ZCASH_NODE_PROTOCOL=http zcash_tx_tool)

[//]: # (```)

[//]: # (The '-it' parameter was added to allow the demo to be interactive.)

