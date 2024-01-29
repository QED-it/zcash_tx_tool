# ZsaWallet

The project uses Diesel ORM framework (https://diesel.rs/) 

To set the database up for the first time:

1) Install diesel_cli: `cargo install diesel_cli --no-default-features --features sqlite`

2) Run migrations: `diesel migration run`