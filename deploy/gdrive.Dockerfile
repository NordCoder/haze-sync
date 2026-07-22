# syntax=docker/dockerfile:1
# Opt-in Google Drive adapter process image.
# Credentials and adapter tokens are mounted at runtime through Compose secrets.

ARG RUST_VERSION=1.88

FROM rust:${RUST_VERSION}-bookworm AS builder
WORKDIR /workspace
COPY . .
RUN cargo build --release --locked -p haze-gdrive-adapter

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 10002 haze-gdrive \
    && useradd --system --uid 10002 --gid 10002 --home-dir /nonexistent --shell /usr/sbin/nologin haze-gdrive

COPY --from=builder /workspace/target/release/haze-gdrive-adapter /usr/local/bin/haze-gdrive-adapter

USER haze-gdrive
ENTRYPOINT ["/usr/local/bin/haze-gdrive-adapter"]
