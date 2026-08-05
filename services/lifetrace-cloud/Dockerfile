# 多阶段构建：runtime 不含 Rust 工具链，非 root 运行。
FROM rust:1.85-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services/lifetrace-cloud ./services/lifetrace-cloud
RUN cargo build --release --manifest-path services/lifetrace-cloud/Cargo.toml

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 lifetrace \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/services/lifetrace-cloud/target/release/lifetrace-cloud /app/lifetrace-cloud
COPY services/lifetrace-cloud/migrations /app/migrations
USER lifetrace
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD ["/app/lifetrace-cloud-healthcheck"] || true
ENTRYPOINT ["/app/lifetrace-cloud"]
