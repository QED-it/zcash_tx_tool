# Match the channel pinned in rust-toolchain.toml so rustup has nothing to install at build time.
FROM rust:1.86.0

# System dependencies
RUN apt-get update && apt-get install -y \
    sqlite3 \
    libsqlite3-dev \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# diesel_cli — independent of project source.
RUN cargo install diesel_cli@2.1.1 --no-default-features --features sqlite --locked

# go-ipfs — needed by fetch-params.sh
RUN wget -q https://dist.ipfs.io/go-ipfs/v0.9.1/go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    tar -xzf go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    cd go-ipfs && \
    bash install.sh && \
    cd .. && \
    rm -rf go-ipfs go-ipfs_v0.9.1_linux-amd64.tar.gz

# Zcash params (~700 MB). Copy just fetch-params.sh first so this layer stays
# cached across source-only changes.
COPY zcutil/fetch-params.sh /app/zcutil/fetch-params.sh
RUN chmod +x /app/zcutil/fetch-params.sh && /app/zcutil/fetch-params.sh
RUN mkdir -p /root/.local/share/ZcashParams

# Project source
COPY . .

# Build
RUN cargo build --release && \
    cp target/release/zcash_tx_tool /app/zcash_tx_tool

# Run migrations (build-time; the runtime app also runs them on /data/walletdb.sqlite)
RUN DATABASE_URL=walletdb.sqlite diesel migration run

# Validate the binary landed where we expect.
RUN test -f /app/zcash_tx_tool

# Default environment variables
ENV ZCASH_NODE_ADDRESS=127.0.0.1
ENV ZCASH_NODE_PORT=18232
ENV ZCASH_NODE_PROTOCOL=http

# Runtime working directory is separate from the build tree so a volume
# mount at /data only shadows the SQLite database, not the binary or
# source. The default DATABASE_URL is the relative `walletdb.sqlite`,
# which resolves to /data/walletdb.sqlite at runtime.
RUN mkdir -p /data
WORKDIR /data

ENTRYPOINT ["/app/zcash_tx_tool"]
CMD ["test-orchard-zsa"]
