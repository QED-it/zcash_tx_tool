# syntax=docker/dockerfile:1.7
# Match the channel pinned in rust-toolchain.toml so rustup has nothing to install at build time.
FROM rust:1.86.0

# System dependencies (cached unless this RUN changes)
RUN apt-get update && apt-get install -y \
    sqlite3 \
    libsqlite3-dev \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# diesel_cli — independent of project source. Cargo registry/git mounts persist the
# crate index and downloaded sources across CI builds.
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    cargo install diesel_cli@2.1.1 --no-default-features --features sqlite --locked

# go-ipfs — needed by fetch-params.sh; layer cached unless the install steps change.
RUN wget -q https://dist.ipfs.io/go-ipfs/v0.9.1/go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    tar -xzf go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    cd go-ipfs && \
    bash install.sh && \
    cd .. && \
    rm -rf go-ipfs go-ipfs_v0.9.1_linux-amd64.tar.gz

# Zcash params (~700 MB). Copying just fetch-params.sh first means the layer is
# cached across source-only changes and only invalidates if the script itself changes.
COPY zcutil/fetch-params.sh /app/zcutil/fetch-params.sh
RUN chmod +x /app/zcutil/fetch-params.sh && /app/zcutil/fetch-params.sh
RUN mkdir -p /root/.local/share/ZcashParams

# Project source. Everything below this layer rebuilds when any tracked file changes,
# but cargo's incremental build (via the /app/target cache mount) keeps it fast.
COPY . .

# Release build. The /app/target cache mount preserves cargo's incremental state
# across CI runs, so source-only changes recompile only the changed crate(s).
# The binary is copied out of the cache mount because cache mounts are not part
# of the resulting image filesystem.
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/app/target,sharing=locked \
    cargo build --release && \
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
