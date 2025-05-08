FROM rust:1.74.0

# Install system dependencies
RUN apt-get update && apt-get install -y \
    sqlite3 \
    libsqlite3-dev \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project and copy only dependency files
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/tx-tool/Cargo.toml ./crates/tx-tool/  # adjust as needed

# Build dependencies only
RUN cargo fetch
RUN cargo build --release || true  # warms up cache even if build fails due to missing sources

# Now copy the rest of the source
COPY . .

# Install diesel_cli
RUN cargo install diesel_cli@2.1.1 --no-default-features --features sqlite --locked

# Run migrations
RUN diesel migration run

# Final build
RUN cargo build --release

# IPFS + fetch params
RUN wget https://dist.ipfs.io/go-ipfs/v0.9.1/go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    tar -xvzf go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    cd go-ipfs && bash install.sh && cd .. && \
    rm -rf go-ipfs go-ipfs_v0.9.1_linux-amd64.tar.gz

RUN chmod +x ./zcutil/fetch-params.sh && ./zcutil/fetch-params.sh

# Set env + entrypoint
ENV ZCASH_NODE_ADDRESS=127.0.0.1
ENV ZCASH_NODE_PORT=18232
ENV ZCASH_NODE_PROTOCOL=http

ENTRYPOINT ["cargo", "run", "--release", "--package", "zcash_tx_tool", "--bin", "zcash_tx_tool", "test-orchard-zsa"]
