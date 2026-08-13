# syntax=docker/dockerfile:1
# Multi-stage build. The runtime image contains no Rust toolchain and runs as
# an unprivileged user.
FROM rust:1.88-slim AS builder
WORKDIR /build
ENV CARGO_NET_RETRY=3 \
    CARGO_HTTP_TIMEOUT=60 \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
COPY crates ./crates
COPY services/cloud ./services/cloud

# Keep Cargo's registry/git cache across BuildKit invocations. Fetching is kept
# separate from compilation so transient network failures do not discard crates
# that were already downloaded; the actual release build then runs offline.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    sh -ec 'for attempt in 1 2 3 4 5; do \
      cargo fetch --locked --manifest-path services/cloud/Cargo.toml && exit 0; \
      echo "cargo fetch failed (attempt ${attempt}/5), retrying in 5s..." >&2; \
      sleep 5; \
    done; exit 1'

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --offline --locked --release \
    --manifest-path services/cloud/Cargo.toml \
    --bin lifetrace-cloud \
    --bin mail_worker

FROM debian:bookworm-slim
RUN sed -i \
        -e 's|deb.debian.org/debian-security|mirrors.aliyun.com/debian-security|g' \
        -e 's|deb.debian.org/debian|mirrors.aliyun.com/debian|g' \
        /etc/apt/sources.list.d/debian.sources \
    && apt-get -o Acquire::Retries=3 update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 lifetrace
WORKDIR /app
COPY --from=builder /build/services/cloud/target/release/lifetrace-cloud /app/lifetrace-cloud
COPY --from=builder /build/services/cloud/target/release/mail_worker /app/mail_worker
USER lifetrace
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:8787/health/ready || exit 1
ENTRYPOINT ["/app/lifetrace-cloud"]
