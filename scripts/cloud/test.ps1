$ErrorActionPreference = "Stop"

$compose = "deploy/cloud/docker-compose.test.yml"
docker compose -f $compose up -d --wait
try {
    $env:TEST_DATABASE_URL = "postgres://lifetrace:lifetrace_test_password@127.0.0.1:5433/lifetrace_test"
    $env:DATABASE_URL = $env:TEST_DATABASE_URL
    cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml
    cargo test --manifest-path services/lifetrace-cloud/Cargo.toml
} finally {
    docker compose -f $compose down -v
}
