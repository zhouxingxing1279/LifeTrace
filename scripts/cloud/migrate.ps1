$ErrorActionPreference = "Stop"

if (-not $env:DATABASE_URL) {
    throw "DATABASE_URL is required"
}

cargo run `
    --manifest-path services/cloud/Cargo.toml `
    --bin lifetrace-migrate
