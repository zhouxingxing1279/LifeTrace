# Multi-stage build. The runtime image contains no Rust toolchain and runs as
# an unprivileged user.
FROM rust:1.88-slim AS builder
WORKDIR /build
ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_TIMEOUT=600
COPY crates ./crates
COPY services/cloud ./services/cloud
RUN cargo build --release \
    --config 'source.crates-io.replace-with="rsproxy-sparse"' \
    --config 'source.rsproxy-sparse.registry="sparse+https://rsproxy.cn/index/"' \
    --manifest-path services/cloud/Cargo.toml \
    --bin lifetrace-cloud \
    --bin mail_worker

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 lifetrace \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/services/cloud/target/release/lifetrace-cloud /app/lifetrace-cloud
COPY --from=builder /build/services/cloud/target/release/mail_worker /app/mail_worker
USER lifetrace
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:8787/health/ready || exit 1
ENTRYPOINT ["/app/lifetrace-cloud"]
