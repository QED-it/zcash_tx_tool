FROM ubuntu:22.04

# Set up Rust and cargo
RUN apt-get update && apt-get install git build-essential clang curl unzip -y

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

# Download and extract the zcash cached chain to /etc/zebra-test
WORKDIR /etc

RUN curl -O https://qedit-public.s3.eu-central-1.amazonaws.com/zsa/zcash-state.zip

RUN unzip zcash-state.zip -d zebra-test

# Checkout and build custom branch of the zebra repository
RUN git clone https://github.com/QED-it/zebra.git

WORKDIR zebra

RUN git switch singlenode-network-txv5

RUN cargo build --release --package zebrad --bin zebrad --features="getblocktemplate-rpcs"

EXPOSE 18232

# Run the zebra node
ENTRYPOINT ["target/release/zebrad", "-c", "singlenode-config.toml"]