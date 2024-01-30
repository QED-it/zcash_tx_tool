# ZsaWallet


# Prerequisites 

The project uses Diesel ORM framework (https://diesel.rs/) 

To set the database up for the first time:

1) Install diesel_cli: `cargo install diesel_cli --no-default-features --features sqlite`

2) Run migrations: `diesel migration run`


# Zebra docker 

activation height

disabled PoW 

docker build -t qedit/zebra-singlenode-txv5 . 

docker run -p 18232:18232 qedit/zebra-singlenode-txv5


# Configuration

The path to the configuration file can be specified with the `--config` flag when running the application. Default filename is "config.toml"

Example configuration file with default values can be found in `example_config.toml`

# Main test scenario

zsa_wallet test