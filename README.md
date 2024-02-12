# Zcash transaction tool

The tool is designed to create and send Zcash transactions to a node (e.g. Zebra). Currently, it supports V5 transactions only.

The repo includes a simple Zebra docker image with a few changes to the original Zebra code to support the tool's test scenario.

Core external building blocks are:

1) Abscissa framework (https://github.com/iqlusioninc/abscissa)

2) Diesel ORM framework (https://diesel.rs/) 

3) librustzcash (https://github.com/zcash/librustzcash) for transaction creation and serialization



# Zebra node 

In our Zebra build there are several changes compared to normal operation:

- Activation height is set to 1.060.755

- PoW is disabled 

- Consensus branch id is set to custom value to fork from main chain

- Peers lists are empty, meaning that a node will not connect to any other nodes

- Blocks up to new activation height are pre-mined and stored in the database that is built into the docker image to avoid long initial sync time

To build and run the docker image:

`docker build -t qedit/zebra-singlenode-txv5 .` 

`docker run -p 18232:18232 qedit/zebra-singlenode-txv5`


# Configuration

The path to the configuration file can be specified with the `--config` flag when running the application. Default filename is "config.toml"

Example configuration file with default values can be found in `example_config.toml`


# Build instructions

To set the Diesel database up:

1) Install diesel_cli: `cargo install diesel_cli --no-default-features --features sqlite`

2) Run migrations: `diesel migration run`

To build the application, simply run:

`cargo build`

although release build is highly recommended for performance reasons:

`cargo build --release`


# Main test scenario

Main test scenario ([src/commands/test.rs](src/commands/test.rs)) consists of the following steps:

1) Mine 100 empty block to be able to use transparent coinbase output
2) Create and mine a new shielding transaction with a transparent input and a shielded output
3) Create and mine a new transfer transaction with shielded inputs and outputs

To run the test scenario:

`cargo run --package zcash_tx_tool --bin zcash_tx_tool test`

with optional, but recommended `--release` flag, or simply 

`zcash_tx_tool test`