# LifeTrace Finance — BeeCount-inspired domain expansion

Status: **implemented and verified**

Branch: `feature/beecount-domain-model`
Reference implementation: `TNT-Likely/BeeCount`
Android client: `zhouxingxing1279/LifeTrace-finance`

## Architecture decision

LifeTrace Cloud remains the **only backend used by native LifeTrace clients**.
The optional production deployment now also includes BeeCount Cloud as a
protocol-compatibility boundary for the already-built BeeCount iOS application;
see `docs/beecount-cloud-integration/`. It has isolated persistence and does not
replace or fork the LifeTrace sync path described below.

The production path remains:

`Android Room -> sync_outbox -> /api/v1/sync/push -> lifetrace-contracts -> PostgreSQL sync_entities/sync_change_log -> /api/v1/sync/pull|snapshot -> Android Room`

The existing authenticated principal, scopes, dependency checks, optimistic versions, conflicts, cursor, snapshot and tombstone semantics are reused unchanged.

## Implemented finance domain

The existing registry now supports these ten finance entity types:

- `finance.ledger`
- `finance.account`
- `finance.category`
- `finance.transaction`
- `finance.recurring_transaction`
- `finance.tag`
- `finance.transaction_tag`
- `finance.budget`
- `finance.transaction_attachment`
- `finance.transaction_evidence`

New BeeCount-inspired entities (`ledger`, recurring transactions, tags, tag relations, budgets and attachment metadata) are strongly typed in `lifetrace-contracts` and registered as user-owned, bidirectional, optimistic-sync entities.

Existing `finance.account`, `finance.category` and `finance.transaction` Rust DTOs deliberately keep their previous public field layout for desktop/source compatibility. Android can send newer forward fields such as `ledgerId`, credit-card metadata, recurrence links, statistics/budget exclusion flags and native-currency snapshots. Serde validation accepts the payload and LifeTrace Cloud persists the original JSON, so these forward fields round-trip without forcing older desktop callers to adopt new Rust struct members immediately.

## Data conventions

LifeTrace conventions take precedence over BeeCount implementation details:

- stable string `EntityId` values, not auto-increment IDs;
- integer cents for money, never floating-point transaction amounts;
- `amountCents` is the transaction/account-currency amount;
- `nativeAmountCents` is the frozen ledger-base-currency snapshot;
- `exchangeRate` is serialized as a decimal string;
- tags use a normalized `finance.transaction_tag` relation;
- transfers remain one transaction with `transactionType=transfer`, `accountId` and `toAccountId`;
- attachment entities hold metadata/file references and reuse LifeTrace's file subsystem rather than creating finance-specific object storage.

## Compatibility

- Existing v1 finance payloads remain readable.
- Unknown Android forward fields are retained in the original sync JSON stored by Cloud.
- New entity descriptors use the existing generic PostgreSQL sync repository; no finance business tables were added.
- Existing push/pull/snapshot, conflict and tombstone algorithms were not forked.
- Frozen v1 capabilities fixtures remain valid; historical entity types must stay a subset of the current registry.

## Scope intentionally not copied from BeeCount

This domain-model pass does not reimplement BeeCount's sync protocol, shared
ledger mirror tables, AI conversation persistence or a separate cloud
exchange-rate service. The isolated compatibility container remains the
BeeCount protocol owner. Phase 2 now adds a Rust read adapter and LifeTrace Web
view without copying BeeCount rows into LifeTrace PostgreSQL; bidirectional
writes and unified persistence remain Phase 3 work.

## Verification result

Final code head before this documentation-only update: `98300e58ddccc6b1a305a04775251ac277917b8d`.

Relevant GitHub Actions results:

- **EPIC-03 PostgreSQL #334 — success**
  - contract tests
  - Cloud tests
  - Clippy production targets
  - Docker build
  - Compose PostgreSQL smoke test
- **EPIC-05 Windows Sync #380 — success**
  - Rustfmt
  - contract tests
  - pure sync-core tests
  - desktop SQLite/adapter tests
  - Clippy sync core + desktop library
  - frontend build
  - Windows core/desktop tests and frontend build
  - Cloud PostgreSQL regression
- **Browser Web #240 — success**

During verification, the existing Rust 1.88 formatting baseline was repaired with formatter-only changes in the affected desktop/cloud test files. No workflow diagnostics or temporary workflow permissions remain in the final PR.

## Acceptance result

The Cloud side is ready for the paired Android implementation: the BeeCount-inspired bookkeeping entities synchronize through the existing LifeTrace Cloud and PostgreSQL sync repository without introducing a second backend.
