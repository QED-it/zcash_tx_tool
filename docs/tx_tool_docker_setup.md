# Running the tx-tool in Docker

The tx-tool is intended to be built and run natively (see the project [README](../README.md)). This document covers the optional Docker workflow used by the CI and by anyone who wants a self-contained tx-tool container.

Zebra is always run in Docker; that side of the workflow lives in the README.

## Build the tx-tool image

### Prerequisites

- Docker (any modern version).
- ~3 GB of free disk space for the resulting image (Rust toolchain + release build + Sapling parameters).
- Network access to `crates.io`, `github.com` (for the QED-it forks of orchard / librustzcash / sapling-crypto), and the IPFS gateway used by `zcutil/fetch-params.sh`.

### Command

From the repository root:

```bash
docker build -t zcash-tx-tool:local .
```

First build typically takes 10–20 minutes (toolchain install, dependency compile, release build, parameter fetch). Subsequent builds reuse Docker's layer cache; only layers downstream of changed files rebuild. The most common iteration ("I changed Rust source") only reruns `cargo build --release`, a few minutes.

### What the image contains

| Path | Content |
|---|---|
| `/app` | Source tree (`COPY . .`) and the build output at `/app/target/release/zcash_tx_tool`. |
| `/data` | Empty at build time. Runtime working directory; the SQLite database lives here. |
| `/root/.local/share/ZcashParams` | Sapling parameters fetched via `zcutil/fetch-params.sh`. |
| `/usr/local/cargo/bin/diesel` | Diesel CLI, used during the build to apply migrations. |

The default entrypoint is `/app/target/release/zcash_tx_tool`, with `CMD ["test-orchard-zsa"]`. Override with any subcommand the binary supports (`test-three-party`, `test-issue-one`, `test-orchard`, `test-persistence-part1`, `test-persistence-part2`, `clean`, `get-block-data`).

### Tag and build arg conventions

`zcash-tx-tool:local` is what this doc and the CI use; nothing depends on the exact tag. The `Dockerfile` carries no `ARG`s — to change the Rust version, edit the `FROM rust:1.86.0` line.

## Run a test scenario against host-side Zebra

```bash
docker run \
  --add-host=host.docker.internal:host-gateway \
  -e ZCASH_NODE_ADDRESS=host.docker.internal \  # or 127.0.0.1 with --network host
  -e ZCASH_NODE_PORT=18232 \
  -e ZCASH_NODE_PROTOCOL=http \
  -v wallet-data:/data \
  zcash-tx-tool:local test-orchard-zsa
```

Assumes Zebra is reachable on the host at port 18232 — the default produced by the Zebra `docker run -p 18232:18232 ...` step in the README. `--add-host=host.docker.internal:host-gateway` is a no-op on macOS/Windows (where the alias already exists) and lets Linux containers reach the host the same way.

## Persistence: mount the wallet state at `/data`

The `-v wallet-data:/data` flag creates a named Docker volume (`wallet-data`) and mounts it at `/data`. The tx-tool writes `walletdb.sqlite` (and any other runtime files) there, so block data and wallet state survive between container runs.

**Without `-v wallet-data:/data`**, the database is written into a writable layer that's discarded when the container is removed — block data and wallet state will not survive between runs.

The mount targets `/data`, not `/app`, deliberately: mounting at `/app` would shadow the binary and source tree on subsequent runs, causing the container to execute a stale binary after image rebuilds.

## Multi-container orchestration

For setups that put Zebra and the tx-tool on the same Docker network with hostname-based addressing (no host networking, no `host.docker.internal`), see the canonical recipe in [`.github/workflows/zebra-test-ci.yaml`](../.github/workflows/zebra-test-ci.yaml). It builds the Zebra image at a pinned commit, brings up `zebra-node` on a user-defined network, and runs the tx-tool container connected to the same network with `ZCASH_NODE_ADDRESS=zebra-node`.
