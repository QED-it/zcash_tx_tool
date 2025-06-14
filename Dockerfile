# Use a more recent Rust version
FROM rust:1.74.0

# Install system dependencies
RUN apt-get update && apt-get install -y \
    sqlite3 \
    libsqlite3-dev \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the entire repository
COPY . .

# Install diesel_cli with specific version
RUN cargo install diesel_cli@2.1.1 --no-default-features --features sqlite --locked

# Run migrations
RUN diesel migration run

# Build the application in release mode
RUN cargo build --release

# Install ipfs (for fetch-params.sh)
RUN wget https://dist.ipfs.io/go-ipfs/v0.9.1/go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    tar -xvzf go-ipfs_v0.9.1_linux-amd64.tar.gz && \
    cd go-ipfs && \
    bash install.sh && \
    cd .. && \
    rm -rf go-ipfs go-ipfs_v0.9.1_linux-amd64.tar.gz

# Make fetch-params.sh executable
RUN chmod +x ./zcutil/fetch-params.sh

# Run fetch-params.sh
RUN ./zcutil/fetch-params.sh

# Create necessary directories
RUN mkdir -p /root/.local/share/ZcashParams

# Set default environment variables
ENV ZCASH_NODE_ADDRESS=127.0.0.1
ENV ZCASH_NODE_PORT=18232
ENV ZCASH_NODE_PROTOCOL=http

# Set the entrypoint with default scenario as "test-orchard-zsa"
ENTRYPOINT ["cargo", "run", "--release", "--package", "zcash_tx_tool", "--bin", "zcash_tx_tool"]
CMD ["test-orchard-zsa"]
