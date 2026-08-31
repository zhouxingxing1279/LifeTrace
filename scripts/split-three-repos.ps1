param(
    [string]$Org = "LifeTraceManage",
    [string]$SourceRepo = "LifeTrace",
    [string]$WebRepo = "LifeTrace-web",
    [string]$DesktopRepo = "LifeTrace-desktop",
    [string]$CloudRepo = "LifeTrace-cloud",
    [ValidateSet("public", "private", "internal")]
    [string]$Visibility = "public",
    [string]$Workspace = "$PWD\.lifetrace-repo-split"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Require-Command([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command '$Name' was not found in PATH."
    }
}

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments
    )
    Push-Location $WorkingDirectory
    try {
        & git @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

function Invoke-Gh {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & gh @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "gh $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Ensure-Repo([string]$FullName) {
    & gh repo view $FullName --json name *> $null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Repository already exists: $FullName"
        return
    }

    Write-Host "Creating repository: $FullName"
    Invoke-Gh repo create $FullName "--$Visibility" --disable-wiki
}

function Export-Subtree {
    param(
        [string]$SourceDir,
        [string]$Prefix,
        [string]$BranchName,
        [string]$OutputDir
    )

    if (Test-Path $OutputDir) {
        Remove-Item -Recurse -Force $OutputDir
    }

    Push-Location $SourceDir
    try {
        & git show-ref --verify --quiet "refs/heads/$BranchName"
        if ($LASTEXITCODE -eq 0) {
            & git branch -D $BranchName | Out-Null
        }
        & git subtree split "--prefix=$Prefix" -b $BranchName
        if ($LASTEXITCODE -ne 0) {
            throw "git subtree split failed for $Prefix"
        }
        & git clone --single-branch --branch $BranchName $SourceDir $OutputDir
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to materialize subtree $Prefix"
        }
    }
    finally {
        Pop-Location
    }

    Invoke-Git $OutputDir remote remove origin
    Invoke-Git $OutputDir branch -M main
}

function Replace-Text {
    param([string]$Path, [string]$Old, [string]$New)
    $content = Get-Content -Raw -LiteralPath $Path
    if (-not $content.Contains($Old)) {
        throw "Expected text was not found in $Path`nOLD: $Old"
    }
    $content = $content.Replace($Old, $New)
    Set-Content -LiteralPath $Path -Value $content -NoNewline -Encoding utf8
}

function Copy-Tree([string]$Source, [string]$Destination) {
    if (Test-Path $Destination) {
        Remove-Item -Recurse -Force $Destination
    }
    $parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    Copy-Item -Recurse -Force -LiteralPath $Source -Destination $Destination
}

function Commit-All([string]$RepoDir, [string]$Message) {
    Invoke-Git $RepoDir add -A
    Push-Location $RepoDir
    try {
        & git diff --cached --quiet
        if ($LASTEXITCODE -eq 0) { return }
    }
    finally {
        Pop-Location
    }
    Invoke-Git $RepoDir commit -m $Message
}

function Push-Repo([string]$RepoDir, [string]$FullName) {
    Invoke-Git $RepoDir remote add origin "https://github.com/$FullName.git"
    Invoke-Git $RepoDir push -u origin main
}

Require-Command git
Require-Command gh
Invoke-Gh auth status

$SourceFullName = "$Org/$SourceRepo"
$WebFullName = "$Org/$WebRepo"
$DesktopFullName = "$Org/$DesktopRepo"
$CloudFullName = "$Org/$CloudRepo"

$Workspace = [System.IO.Path]::GetFullPath($Workspace)
$SourceDir = Join-Path $Workspace "source"
$WebDir = Join-Path $Workspace "web"
$DesktopDir = Join-Path $Workspace "desktop"
$CloudDir = Join-Path $Workspace "cloud"

if (Test-Path $Workspace) {
    Remove-Item -Recurse -Force $Workspace
}
New-Item -ItemType Directory -Force -Path $Workspace | Out-Null

Write-Host "Cloning $SourceFullName ..."
& git clone "https://github.com/$SourceFullName.git" $SourceDir
if ($LASTEXITCODE -ne 0) { throw "Failed to clone $SourceFullName" }
Invoke-Git $SourceDir checkout main
Invoke-Git $SourceDir pull --ff-only origin main

Push-Location $SourceDir
try { $SourceRevision = (& git rev-parse HEAD).Trim() }
finally { Pop-Location }
Write-Host "Source revision: $SourceRevision"

# -----------------------------------------------------------------------------
# 1) Web: standalone subtree with history
# -----------------------------------------------------------------------------
Write-Host "`n=== Web ==="
Export-Subtree $SourceDir "apps/web" "repo-split/web" $WebDir

New-Item -ItemType Directory -Force -Path (Join-Path $WebDir ".github\workflows") | Out-Null
$webCi = @'
name: CI

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

jobs:
  web:
    runs-on: ubuntu-latest
    timeout-minutes: 25
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: package.json
      - run: npm install --no-audit --no-fund
      - run: npm run typecheck
      - run: npm test
      - run: npm run build
      - name: Verify artifact
        shell: bash
        run: |
          test -f dist/index.html
          test -f dist/vendor/vditor/dist/js/lute/lute.min.js
'@
Set-Content -LiteralPath (Join-Path $WebDir ".github\workflows\ci.yml") -Value $webCi -Encoding utf8
Set-Content -LiteralPath (Join-Path $WebDir "MIGRATED_FROM.md") -Value "Extracted from $SourceFullName at $SourceRevision.`n" -Encoding utf8
Commit-All $WebDir "chore: make web repository standalone"
Ensure-Repo $WebFullName
Push-Repo $WebDir $WebFullName

# -----------------------------------------------------------------------------
# 2) Cloud: server subtree + shared contracts/sync-client ownership
# -----------------------------------------------------------------------------
Write-Host "`n=== Cloud ==="
Export-Subtree $SourceDir "services/cloud" "repo-split/cloud" $CloudDir

Copy-Tree (Join-Path $SourceDir "crates\lifetrace-contracts") (Join-Path $CloudDir "crates\lifetrace-contracts")
Copy-Tree (Join-Path $SourceDir "crates\lifetrace-sync-client") (Join-Path $CloudDir "crates\lifetrace-sync-client")
Copy-Tree (Join-Path $SourceDir "contracts") (Join-Path $CloudDir "contracts")
if (Test-Path (Join-Path $SourceDir "tools\contract-exporter")) {
    Copy-Tree (Join-Path $SourceDir "tools\contract-exporter") (Join-Path $CloudDir "tools\contract-exporter")
}

Replace-Text (Join-Path $CloudDir "Cargo.toml") 'lifetrace-contracts = { path = "../../crates/lifetrace-contracts" }' 'lifetrace-contracts = { path = "crates/lifetrace-contracts" }'

$cloudDocker = @'
# syntax=docker/dockerfile:1
FROM rust:1.88-slim AS builder
WORKDIR /build
ENV CARGO_NET_RETRY=3 \
    CARGO_HTTP_TIMEOUT=60 \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    sh -ec 'for attempt in 1 2 3 4 5; do cargo fetch --locked && exit 0; sleep 5; done; exit 1'
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --offline --locked --release \
      --bin lifetrace-cloud \
      --bin mail_worker \
      --bin execution_worker \
      --bin lifetrace-migrate \
      --bin lifetrace-admin

FROM debian:bookworm-slim
RUN sed -i \
      -e 's|deb.debian.org/debian-security|mirrors.aliyun.com/debian-security|g' \
      -e 's|deb.debian.org/debian|mirrors.aliyun.com/debian|g' \
      /etc/apt/sources.list.d/debian.sources \
    && apt-get -o Acquire::Retries=3 update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 lifetrace \
    && mkdir -p /data/photo-staging \
    && chown -R lifetrace:lifetrace /data
WORKDIR /app
COPY --from=builder /build/target/release/lifetrace-cloud /app/lifetrace-cloud
COPY --from=builder /build/target/release/mail_worker /app/mail_worker
COPY --from=builder /build/target/release/execution_worker /app/execution_worker
COPY --from=builder /build/target/release/lifetrace-migrate /app/lifetrace-migrate
COPY --from=builder /build/target/release/lifetrace-admin /app/lifetrace-admin
USER lifetrace
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD curl --fail --silent http://127.0.0.1:8787/health/ready || exit 1
ENTRYPOINT ["/app/lifetrace-cloud"]
'@
Set-Content -LiteralPath (Join-Path $CloudDir "Dockerfile") -Value $cloudDocker -Encoding utf8

New-Item -ItemType Directory -Force -Path (Join-Path $CloudDir ".github\workflows") | Out-Null
$cloudCi = @'
name: CI

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

jobs:
  cloud:
    runs-on: ubuntu-latest
    timeout-minutes: 50
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_USER: lifetrace
          POSTGRES_PASSWORD: lifetrace_test_password
          POSTGRES_DB: lifetrace_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U lifetrace -d lifetrace_test"
          --health-interval 5s
          --health-timeout 3s
          --health-retries 20
    env:
      DATABASE_URL: postgres://lifetrace:lifetrace_test_password@127.0.0.1:5432/lifetrace_test
      TEST_DATABASE_URL: postgres://lifetrace:lifetrace_test_password@127.0.0.1:5432/lifetrace_test
      AUTH_PASSWORD_PEPPER: ci-password-pepper
      AUTH_TOKEN_HASH_PEPPER: ci-token-pepper
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.88.0
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo test --locked -- --test-threads=1
      - run: cargo clippy --locked --all-targets -- -D warnings
'@
Set-Content -LiteralPath (Join-Path $CloudDir ".github\workflows\ci.yml") -Value $cloudCi -Encoding utf8
Set-Content -LiteralPath (Join-Path $CloudDir "MIGRATED_FROM.md") -Value "Extracted from $SourceFullName at $SourceRevision. This repository owns lifetrace-contracts and lifetrace-sync-client after the split.`n" -Encoding utf8
Commit-All $CloudDir "chore: make cloud repository standalone and own shared contracts"
Ensure-Repo $CloudFullName
Push-Repo $CloudDir $CloudFullName

# -----------------------------------------------------------------------------
# 3) Desktop: standalone subtree with explicit Web + Cloud dependencies
# -----------------------------------------------------------------------------
Write-Host "`n=== Desktop ==="
Export-Subtree $SourceDir "apps/desktop" "repo-split/desktop" $DesktopDir

Invoke-Git $DesktopDir submodule add "https://github.com/$WebFullName.git" "vendor/web"
Invoke-Git $DesktopDir submodule add "https://github.com/$CloudFullName.git" "vendor/cloud"

Replace-Text (Join-Path $DesktopDir "src-tauri\Cargo.toml") 'lifetrace-contracts = { path = "../../../crates/lifetrace-contracts" }' 'lifetrace-contracts = { path = "../vendor/cloud/crates/lifetrace-contracts" }'
Replace-Text (Join-Path $DesktopDir "src-tauri\Cargo.toml") 'lifetrace-sync-client = { path = "../../../crates/lifetrace-sync-client" }' 'lifetrace-sync-client = { path = "../vendor/cloud/crates/lifetrace-sync-client" }'
Replace-Text (Join-Path $DesktopDir "tsconfig.json") '"../../contracts/typescript/**/*.ts"' '"vendor/cloud/contracts/typescript/**/*.ts"'
Replace-Text (Join-Path $DesktopDir "tsconfig.json") '"../web/src/**/*.ts"' '"vendor/web/src/**/*.ts"'
Replace-Text (Join-Path $DesktopDir "tsconfig.json") '"../web/src/**/*.tsx"' '"vendor/web/src/**/*.tsx"'
Replace-Text (Join-Path $DesktopDir "tauri-ui\main.tsx") 'import "../../web/src/styles/globals.css";' 'import "../vendor/web/src/styles/globals.css";'
Replace-Text (Join-Path $DesktopDir "src\components\DesktopCloudWorkspace.tsx") 'from "../../../web/src/app/AppContext";' 'from "../../vendor/web/src/app/AppContext";'
Replace-Text (Join-Path $DesktopDir "src\components\DesktopCloudWorkspace.tsx") 'from "../../../web/src/app/DesktopFeatureRouter";' 'from "../../vendor/web/src/app/DesktopFeatureRouter";'
Replace-Text (Join-Path $DesktopDir "src\components\DesktopCloudWorkspace.tsx") 'from "../../../web/src/services/core";' 'from "../../vendor/web/src/services/core";'
Replace-Text (Join-Path $DesktopDir "scripts\ensure-shared-web-deps.mjs") 'const webRoot = path.resolve(scriptDir, "../../web");' 'const webRoot = path.resolve(scriptDir, "../vendor/web");'
Replace-Text (Join-Path $DesktopDir "vite.tauri.config.ts") 'const appsRoot = path.resolve(projectRoot, "..");' 'const webRoot = path.resolve(projectRoot, "vendor", "web");'
Replace-Text (Join-Path $DesktopDir "vite.tauri.config.ts") 'postcss: path.join(appsRoot, "web", "postcss.config.cjs"),' 'postcss: path.join(webRoot, "postcss.config.cjs"),'
Replace-Text (Join-Path $DesktopDir "vite.tauri.config.ts") 'allow: [appsRoot],' 'allow: [projectRoot, webRoot],'

New-Item -ItemType Directory -Force -Path (Join-Path $DesktopDir ".github\workflows") | Out-Null
$desktopCi = @'
name: CI

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

jobs:
  desktop:
    runs-on: windows-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: package-lock.json
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.88.0
      - run: npm ci
      - run: npm run prepare:web-shared
      - run: npm run lint
      - run: npm run test:unit
      - run: npm run web:build
      - run: npm run test:rust
'@
Set-Content -LiteralPath (Join-Path $DesktopDir ".github\workflows\ci.yml") -Value $desktopCi -Encoding utf8
Set-Content -LiteralPath (Join-Path $DesktopDir "MIGRATED_FROM.md") -Value "Extracted from $SourceFullName at $SourceRevision. Web feature code is pinned through vendor/web; Rust contracts and sync client are pinned through vendor/cloud.`n" -Encoding utf8
Commit-All $DesktopDir "chore: make desktop repository standalone with explicit web and cloud dependencies"
Ensure-Repo $DesktopFullName
Push-Repo $DesktopDir $DesktopFullName

Write-Host ""
Write-Host "Split completed:"
Write-Host "  https://github.com/$WebFullName"
Write-Host "  https://github.com/$DesktopFullName"
Write-Host "  https://github.com/$CloudFullName"
Write-Host ""
Write-Host "Do not delete the old directories from $SourceFullName yet."
Write-Host "Wait for all three new repositories to pass CI, then remove apps/web, apps/desktop and services/cloud in a separate cleanup PR."
