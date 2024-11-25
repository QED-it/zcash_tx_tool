# Zcash transaction tool

The tool is designed to create and send Zcash transactions to a node (e.g., Zebra). Currently, it supports V5 and v6 transactions.

The repo includes a simple Zebra docker image with a few changes to the original Zebra code to support the tool's test scenario.

Core components include:

1) librustzcash for transaction creation and serialization [[Link](https://github.com/zcash/librustzcash)]. Slightly modified with additional functionality.

2) Diesel ORM framework [[Link](https://diesel.rs/)] 

3) Abscissa framework [[Link](https://github.com/iqlusioninc/abscissa)]



## Executing the ZSA flow against a dockerized Zebra node 

To build and run the Zebra docker image:

```bash
% docker build -t qedit/zebra-regtest-txv6 .

% docker run -p 18232:18232 qedit/zebra-regtest-txv6
``` 
More details on how the docker file is created and synced: [Dockerfile](./Dockerfile)

In a different window, setup and run the Zcash transaction tool:

A one time setup for Diesel is required:
```bash
% cargo install diesel_cli --no-default-features --features sqlite

% diesel setup
```
Build and run the orchardZSA test case using the Zcash transaction tool:
```bash
% RUSTFLAGS='--cfg zcash_unstable="nu6"' cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-orchard-zsa
```
To re-run the test scenario, you need to reset the Zebra node by shutting down the Zebra container and starting it again.

The detailed script for the ZSA flow are in the file [test_orchard_zsa.rs](src/commands/test_orchard_zsa.rs)



## Configuration

The path to the configuration file can be specified with the `--config` flag when running the application. The default filename is "config.toml"

An example configuration file with default values can be found in `regtest_config.toml`


## Build instructions

To set the Diesel database up:

1) Install diesel_cli: `cargo install diesel_cli --no-default-features --features sqlite`

2) Set up database: `diesel setup`

To build the application, run:

`cargo build`

Although release build is highly recommended for performance reasons:

`cargo build --release`

To test ZSA functionality with the tool, the corresponding flag should be set:

```bash
% RUSTFLAGS='--cfg zcash_unstable="nu6"' cargo build
```

## ZSA Orchard test scenario

Main test scenario ([src/commands/test_orchard_zsa.rs](src/commands/test_orchard_zsa.rs)) consists of the following steps:

1) Issue an asset
2) Transfer the asset to another account
3) Burn the asset (x2)

To run the test scenario:

```bash
% RUSTFLAGS='--cfg zcash_unstable="nu6"' cargo run --release --package zcash_tx_tool --bin zcash_tx_tool test-orchard-zsa
```

[//]: # ()
[//]: # (## Docker-based demo)

[//]: # ()
[//]: # (You can also run the tests using docker. To do that you'll need first to build the docker image)

[//]: # ()
[//]: # (```bash)

[//]: # (% docker build -t zcash_tx_tool -f Dockerfile-demo .)

[//]: # (```)

[//]: # ()
[//]: # (And after that run the image itself.)

[//]: # (The default connection parameters are set to connect to the zebra-node running on the machine itself &#40;127.0.0.1&#41;)

[//]: # (If you ran the node in a docker container with the command above, you named that container "zebra-node", so you should use that as the ZCASH_NODE_ADDRESS.)

[//]: # (If the node is running on the ECS server, you can connect to it by setting the ZCASH_NODE_ADDRESS=<Domain>.)

[//]: # ()
[//]: # (First, make sure you created the network:)

[//]: # (```bash)

[//]: # (% docker network create zcash-network)

[//]: # (```)

[//]: # (And started the node with the network argument, like this)

[//]: # (```bash)

[//]: # (% docker run --name zebra-node --network zcash-network -p 18232:18232 qedit/zebra-regtest-txv6)

[//]: # (```)

[//]: # ()
[//]: # (Here are the 3 options &#40;No parameters will default to the first configuration&#41;)

[//]: # ()
[//]: # (```bash)

[//]: # (% docker run -it --network zcash-network -e ZCASH_NODE_ADDRESS=127.0.0.1 -e ZCASH_NODE_PORT=18232 -e ZCASH_NODE_PROTOCOL=http zcash_tx_tool)

[//]: # (% docker run -it --network zcash-network -e ZCASH_NODE_ADDRESS=zebra-node -e ZCASH_NODE_PORT=18232 -e ZCASH_NODE_PROTOCOL=http zcash_tx_tool)

[//]: # (% docker run -it --network zcash-network -e ZCASH_NODE_ADDRESS=<Domain> -e ZCASH_NODE_PORT=18232 -e ZCASH_NODE_PROTOCOL=http zcash_tx_tool)

[//]: # (```)

[//]: # (The '-it' parameter was added to allow the demo to be interactive.)
