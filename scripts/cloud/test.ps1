$ErrorActionPreference = "Stop"
docker compose -f deploy/cloud/docker-compose.test.yml up -d
try {
    cargo test --manifest-path services/lifetrace-cloud/Cargo.toml
} finally {
    docker compose -f deploy/cloud/docker-compose.test.yml down
}
