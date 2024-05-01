# Zcash transaction tool

The tool is designed to create and send Zcash transactions to a node (e.g., Zebra). Currently, it supports V5 transactions only.

The repo includes a simple Zebra docker image with a few changes to the original Zebra code to support the tool's test scenario.

Core components include:

1) librustzcash for transaction creation and serialization [[Link](https://github.com/zcash/librustzcash)]. Slightly modified with additional functionality.

2) Diesel ORM framework [[Link](https://diesel.rs/)] 

3) Abscissa framework [[Link](https://github.com/iqlusioninc/abscissa)]



## Zebra node 

Before testing, we need to bring up the desired node and ensure that V5 transactions are activated (NU5 is active).

Currently, we use a custom Zebra build. There are several changes compared to the upstream node:

- Activation height is set to `1,060,755`

- PoW is disabled 

- Consensus branch id is set to custom value. This is done to fork from the main chain

- Peers lists are empty, meaning that a node will not connect to any other nodes

- Blocks up to new activation height are pre-mined and stored in the database that is built into the docker image to avoid long initial sync time

To build and run the docker image:

```bash
docker build -t qedit/zebra-singlenode-txv5 .

docker run -p 18232:18232 qedit/zebra-singlenode-txv5
``` 

More details on how the docker file is created and synced: [Link](https://github.com/QED-it/zcash_tx_tool/blob/main/Dockerfile)


## Configuration

The path to the configuration file can be specified with the `--config` flag when running the application. The default filename is "config.toml"

An example configuration file with default values can be found in `example_config.toml`


## Build instructions

To set the Diesel database up:

1) Install diesel_cli: `cargo install diesel_cli --no-default-features --features sqlite`

2) Run migrations: `diesel migration run`

To build the application, run:

```cargo build```

Although release build is highly recommended for performance reasons:

`cargo build --release`


## Main test scenario

Main test scenario ([src/commands/test.rs](src/commands/test_v5)) consists of the following steps:

1) Mine 100 empty blocks to be able to use transparent coinbase output
2) Create and mine a new shielding transaction with a transparent input and a shielded output
3) Create and mine a new transfer transaction with shielded inputs and outputs
4) Assert balances for the selected accounts

To run the test scenario:

```bash
cargo run --package zcash_tx_tool --bin zcash_tx_tool test
```

With optional, but recommended `--release` flag, or simply 

```bash
zcash_tx_tool test
```
