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
# Build the Zebra Docker image
docker build -t qedit/zebra-regtest-txv6 -f Dockerfile-zebra .

# Run the Zebra Docker container
docker run -p 18232:18232 qedit/zebra-regtest-txv6
```

For more details on how the Docker image is created and synchronized, refer to the [Dockerfile-zebra](./Dockerfile-zebra).

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

## Connecting to the Public ZSA Testnet

We’ve made it easy to test Zcash Shielded Assets (ZSA) functionality by connecting directly to our public Zebra node, hosted by QEDIT on AWS.

### Public Testnet Details

- **TLS-enabled URL**: [https://dev.zebra.zsa-test.net](https://dev.zebra.zsa-test.net)

You can run the `zcash_tx_tool` in a Docker container and connect to our testnet by supplying the appropriate environment variables.

### Usage with Docker (HTTPS)

```bash
docker run --pull=always \
  -e ZCASH_NODE_ADDRESS=dev.zebra.zsa-test.net \
  -e ZCASH_NODE_PORT=443 \
  -e ZCASH_NODE_PROTOCOL=https \
  public.ecr.aws/j7v0v6n9/tx-tool:latest
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

