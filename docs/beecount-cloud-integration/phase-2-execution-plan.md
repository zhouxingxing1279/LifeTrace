# Phase 2 — BeeCount finance read adapter

Status: implemented; server rollout verification pending

## Objective

Make finance data written by the stock BeeCount iOS client readable through the
authenticated LifeTrace Cloud API, so LifeTrace Desktop, Browser and assistant
features can consume it without speaking the BeeCount protocol directly.

## Boundary

This phase is deliberately read-only:

- BeeCount Cloud remains the source of truth for the stock iOS client's data.
- The adapter calls only BeeCount `GET` endpoints after authentication.
- Returned objects are marked `source=beecount-cloud` and `readOnly=true`.
- BeeCount rows are not inserted into LifeTrace `sync_entities` and cannot be
  edited through LifeTrace finance CRUD.
- Bidirectional writes, conflict convergence and unified PostgreSQL persistence
  remain Phase 3 work.

## API

- `GET /api/v1/integrations/beecount/status`
- `GET /api/v1/integrations/beecount/ledgers`
- `GET /api/v1/integrations/beecount/ledgers/{ledger_id}/snapshot?limit=&offset=`

The snapshot contains one normalized ledger, a transaction page, accounts,
categories, tags and budgets. Money is converted to integer cents at the
LifeTrace boundary. BeeCount identifiers are namespaced with `beecount:` to
prevent collisions with native LifeTrace entities.

## LifeTrace Web integration

LifeTrace Web exposes the adapter at `/finance/beecount` inside its existing
authenticated shell. The page includes:

- BeeCount ledger selection and adapter/upstream status;
- ledger balance, income, expense and entity-count overview;
- paged transactions with current-page text/type filtering;
- account balances, categories, tags and budgets;
- LifeTrace privacy-mode masking and responsive desktop/mobile layouts.

The existing LifeTrace finance pages remain native and writable. BeeCount rows
are shown in a separate, visibly read-only area so a user cannot accidentally
edit one store while believing they are editing the other.

## Authentication and isolation

- LifeTrace callers need `finance:read`.
- The integration is bound to exactly one configured LifeTrace user ID; another
  valid LifeTrace account receives `403` and cannot read the shared upstream
  credential's data.
- BeeCount credentials live only in deployment secrets/environment variables.
- The adapter keeps the short-lived BeeCount access token in process memory and
  never returns or logs credentials/tokens.
- Upstream URLs are operator configuration, never caller-controlled; ledger IDs
  are URL encoded before use.
- Response size and timeout limits prevent the upstream from exhausting the
  LifeTrace process.

## Failure model

- Adapter disabled or unreachable: `503 LIFETRACE_TEMPORARILY_UNAVAILABLE`.
- Invalid BeeCount credentials or 2FA-only login: sanitized `502`; no upstream
  response body or secret is exposed.
- Malformed/oversized BeeCount response: sanitized `502`.
- A BeeCount failure does not affect native LifeTrace routes or startup.

## Verification

- configuration validation tests;
- bound-user authorization test;
- upstream authentication/token reuse test;
- ledger and snapshot normalization tests;
- exact decimal-to-cents tests;
- upstream 401 retry and sanitized unavailable-response tests;
- browser navigation, transaction filtering and adapter-client tests;
- existing Cloud, contract and desktop regressions.
