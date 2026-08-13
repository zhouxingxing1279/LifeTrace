# BeeCount Cloud deployment integration

Status: implemented

## Goal

Allow the already-built BeeCount iOS application to connect to the same server
that hosts LifeTrace without changing or rebuilding the iOS client.

## Architecture

- LifeTrace Desktop, Web and Android continue to use LifeTrace Cloud's Rust/Axum
  API and PostgreSQL sync store.
- The stock BeeCount iOS client connects to a dedicated BeeCount hostname and
  uses the unmodified BeeCount Cloud protocol.
- Both services are deployed by `deploy/cloud/docker-compose.production.yml`,
  share Caddy/TLS and the private Docker network, and expose no database or
  application port directly to the public network.
- BeeCount persistent state is isolated in the `beecount_data` Docker volume.

Phase 1 provides protocol compatibility at the deployment boundary. Phase 2
adds a read-only LifeTrace adapter and an integrated Web view, while keeping
BeeCount's SQLite data and LifeTrace's PostgreSQL data isolated. Bidirectional
writes and unified persistence remain Phase 3 work.

## Implemented changes

- Add the official `sunxiao0721/beecount-cloud` image to production Compose.
- Add a persistent `/data` volume and `/ready` health check.
- Disable public registration by default.
- Route a configurable HTTPS hostname through Caddy.
- Keep the BeeCount container internal; only Caddy publishes ports 80/443.
- Provide an environment template and deployment/backup instructions.
- Add a regression test for routing, health checks, persistence and port
  isolation.
- Add authenticated LifeTrace endpoints for BeeCount ledgers and normalized
  ledger snapshots.
- Add the `/finance/beecount` read-only experience to LifeTrace Web.

## Acceptance criteria

- `docker compose config` succeeds with the production environment.
- Caddy and the existing LifeTrace site can start even if BeeCount is still
  migrating; BeeCount's own readiness check reports when its route is usable.
- `https://<BEECOUNT_DOMAIN>/ready` returns `{"status":"ready"}`.
- The stock BeeCount iOS app can log in using that HTTPS origin.
- Existing LifeTrace API and browser routes remain unchanged.
- Recreating the BeeCount container preserves accounts, ledgers and attachments.
