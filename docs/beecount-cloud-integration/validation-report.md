# BeeCount Cloud integration validation

Date: 2026-08-13

## Passed

- TypeScript type check: `npm --prefix apps/desktop run lint`
- Deployment regression tests: 2 passed
- Complete desktop unit suite: 120 passed, 0 failed
- Tauri Web production build: passed
- Browser Web production build: passed
- Rust 1.88 formatting check for all changed Rust files: passed
- Rust 1.88 `cargo check --tests --locked`: passed
- Rust cloud test suite: 105 passed, 0 failed
- Git whitespace validation: `git diff --check`

The deployment tests assert that:

- BeeCount Cloud uses the official image;
- `/data` is backed by the `beecount_data` volume;
- readiness is checked on `/ready`;
- public registration defaults to disabled;
- no BeeCount application port is published directly;
- Caddy proxies the dedicated hostname without making LifeTrace availability
  depend on BeeCount readiness.

The Web/adapter tests additionally assert that:

- LifeTrace Web includes a navigable BeeCount Cloud finance page;
- transaction filtering uses BeeCount note/account/category/tag metadata;
- adapter requests use the LifeTrace cookie session and safely encode ledger
  identifiers;
- another LifeTrace user is denied before any BeeCount login occurs;
- one BeeCount access token is reused and a rejected token is refreshed once;
- upstream failures return sanitized errors without credentials.

## Phase 3 foundation checks

The first Phase 3 batch adds source-level tests for:

- BeeCount application scopes excluding unrelated LifeTrace domains;
- BeeCount snake_case 2FA status output;
- entity type/scope mapping and reversible `beecount:` ID namespacing;
- exact decimal/exponent amount conversion to signed 64-bit cents using
  round-half-away-from-zero;
- the five-second future-clock clamp and `(updated_at, device_id)` LWW order.

The core-sync batch additionally adds a PostgreSQL-backed end-to-end test for:

- stock-client snake_case registration and bearer authentication;
- transactional ledger/account/transaction push into LifeTrace's canonical
  `sync_entities` and `sync_change_log`;
- exact amount round-trip, pull cursor output, ledger listing and v6 full
  snapshot generation;
- idempotent replay and rejection of an older LWW change;
- native LifeTrace finance writes returning through the BeeCount boundary with
  reversible `lifetrace:` wire IDs.

Migration `0017_beecount_compatibility.sql` also passed whitespace, statement
boundary and balanced-parenthesis static checks. The Rust compatibility files
pass Rust 1.88 compilation and the complete cloud test suite.

## Phase 4 attachment and realtime checks

The fourth-stage batch adds and validates:

- multipart transaction attachment and category-icon upload routes;
- per-ledger and per-user SHA-256 deduplication;
- ordered batch-exists responses and authenticated byte downloads;
- atomic `cloud_file_blobs` + `file.metadata` + sync change-log persistence;
- safe path stripping, bounded UTF-8 names and RFC 5987 download headers;
- BeeCount token WebSocket authentication, 1008 policy close, JSON/protocol
  ping/pong and stock camelCase `sync_change` event fields;
- realtime publication after both BeeCount compatibility push and native
  LifeTrace finance push;
- a bounded 64 MiB default upload configuration and route-local body limit.

Migration `0018_beecount_attachments.sql`, the attachment PostgreSQL test and
all WebSocket/attachment unit targets compile under the locked dependency set.
The full Rust run reported 102 passed and 0 failed; the conditional PostgreSQL
test bodies return early when `TEST_DATABASE_URL` is absent, so a real database
run remains a rollout gate.

## Phase 5 profile, device and collaboration checks

The fifth-stage batch adds and validates:

- stock profile GET/PATCH, bounded avatar upload/download and `profile_change`;
- BeeCount device detail persistence, listing and full session/token revocation;
- owner/editor memberships, one-time invites, member removal and ownership transfer;
- member-aware ledger listing, push, pull and full snapshot reads without duplicating
  financial entities into the editor's user partition;
- owner user-global shared-resource snapshots and `shared_resource_change` fan-out;
- shared-ledger attachment authorization and ledger-wide `sync_change` fan-out.

Migration `0019_beecount_profile_collaboration.sql` and the new two-user PostgreSQL
acceptance target compile under Rust 1.88. The complete Rust run reported 105 passed
and 0 failed. As with earlier PostgreSQL targets, the real database body returns early
without `TEST_DATABASE_URL` and remains a rollout gate.

## Environment limitation

The execution environment did not provide Docker or PostgreSQL. A temporary
Rust 1.88 toolchain was assembled from runtime components, so formatting,
locked compilation and all non-database Rust tests were executed locally.
The first server rollout must still run:

- `TEST_DATABASE_URL=... cargo test --locked --manifest-path services/cloud/Cargo.toml`;
- `docker compose ... config` and `up -d --wait`;
- the documented LifeTrace adapter, attachment, WebSocket and BeeCount HTTPS
  smoke checks.

The interactive browser preview was also unavailable because the local preview
runtime could not enumerate network interfaces. Both production Web builds,
all 120 Node tests and all 105 reported Rust tests completed successfully, but
visual and real-PostgreSQL QA must be repeated before protocol cutover.
