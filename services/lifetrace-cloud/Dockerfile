# Multi-stage build. The runtime image contains no Rust toolchain and runs as
# an unprivileged user.
FROM rust:1.88-slim AS builder
WORKDIR /build
ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_TIMEOUT=600
COPY crates ./crates
COPY services/lifetrace-cloud ./services/lifetrace-cloud
RUN cargo build --release \
    --config 'source.crates-io.replace-with="rsproxy-sparse"' \
    --config 'source.rsproxy-sparse.registry="sparse+https://rsproxy.cn/index/"' \
    --manifest-path services/lifetrace-cloud/Cargo.toml \
    --bin lifetrace-cloud

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 lifetrace \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/services/lifetrace-cloud/target/release/lifetrace-cloud /app/lifetrace-cloud
USER lifetrace
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:8787/health/ready || exit 1
ENTRYPOINT ["/app/lifetrace-cloud"]
