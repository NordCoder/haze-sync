# syntax=docker/dockerfile:1
# Local/prod-like packaging scaffold for haze-sync-server.
#
# This image contains no secrets and does not run migrations. Runtime config is
# supplied by environment variables from deployment tooling.

# Rust 1.88 is the minimum toolchain accepted by the canonical locked dependency
# graph. Keep this value aligned with workspace.package.rust-version.
ARG RUST_VERSION=1.88

FROM rust:${RUST_VERSION}-bookworm AS builder
WORKDIR /workspace
COPY . .
RUN cargo build --release --locked -p haze-sync-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --uid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin haze-sync \
    && mkdir -p /var/lib/haze-sync/objects \
    && chown -R haze-sync:haze-sync /var/lib/haze-sync

COPY --from=builder /workspace/target/release/haze-sync-server /usr/local/bin/haze-sync-server

USER haze-sync
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/haze-sync-server"]
