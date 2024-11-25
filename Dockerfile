FROM rust:1.81.0

# Set up Rust and cargo
RUN apt-get update && apt-get install git build-essential clang -y

# Checkout and build custom branch of the zebra repository
RUN git clone https://github.com/QED-it/zebra.git

WORKDIR zebra

RUN git switch zsa-integration-demo

RUN cargo build --release --package zebrad --bin zebrad --features="getblocktemplate-rpcs"

EXPOSE 18232

COPY regtest-config.toml regtest-config.toml

# Run the zebra node
ENTRYPOINT ["target/release/zebrad", "-c", "regtest-config.toml"]
