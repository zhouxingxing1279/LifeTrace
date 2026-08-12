# LifeTrace Finance — BeeCount-inspired domain expansion

Status: execution plan
Branch: `feature/beecount-domain-model`
Reference implementation: `TNT-Likely/BeeCount`
Android client: `zhouxingxing1279/LifeTrace-finance`

## 1. Hard architectural constraint

LifeTrace Cloud remains the only backend. This change **does not introduce a new finance cloud, authentication system, sync protocol, database service, or BeeCount Cloud dependency**.

All new finance entities use the existing LifeTrace pipeline:

`Android Room -> sync_outbox -> /api/v1/sync/push -> lifetrace-contracts registry/payload validation -> PostgreSQL sync_entities + sync_change_log -> /api/v1/sync/pull|snapshot -> Android Room`

The existing authenticated principal, scopes, conflict handling, cursor, snapshot and tombstone semantics are retained.

## 2. Goal

Reproduce the useful single-user bookkeeping domain of BeeCount while preserving LifeTrace wire conventions (`EntityMeta`, integer cents, ISO currency codes, typed entity registry, optimistic server versions).

### In scope

- ledgers/books
- richer accounts
- hierarchical categories
- richer transactions and transfers
- recurring transactions
- tags and transaction-tag relations
- budgets
- transaction attachments/evidence metadata
- multi-currency transaction snapshots
- exclude-from-statistics and exclude-from-budget flags
- hidden/archive ordering metadata
- existing smart capture/import provenance

### Explicitly out of scope for this pass

- BeeCount Cloud
- BeeCount authentication
- shared/family ledger collaboration and member mirror tables
- BeeCount AI conversation/message persistence
- exchange-rate cache as a cloud entity (derived cache remains client-local)

## 3. Cloud entity model

Existing entity types remain backward-compatible and are extended only with optional/defaultable fields where possible.

| Entity type | Purpose | Strategy |
| --- | --- | --- |
| `finance.ledger` | Book/ledger, base currency, month start day | new typed entity |
| `finance.account` | Wallet/bank/credit/cash account | extend existing payload |
| `finance.category` | expense/income hierarchy | extend existing payload |
| `finance.transaction` | expense/income/transfer/refund | extend existing payload |
| `finance.recurring_transaction` | recurring template/rule | new typed entity |
| `finance.tag` | user tag | new typed entity |
| `finance.transaction_tag` | many-to-many relation | new typed entity |
| `finance.budget` | total/category budget | new typed entity |
| `finance.transaction_attachment` | attachment metadata linked to a transaction | new typed entity |
| `finance.transaction_evidence` | capture/import provenance | retain existing entity |

All IDs are stable LifeTrace `EntityId` strings; the Android client must not introduce BeeCount auto-increment IDs as sync identity.

## 4. Field mapping decisions

### Ledger

`meta, name, currency, ledgerType, monthStartDay, sortOrder, isArchived`

For this pass `ledgerType` is `personal`; collaborative roles/member counts are intentionally excluded.

### Account

Retain existing fields and add:

`ledgerId, sortOrder, creditLimitCents, billingDay, paymentDueDay, bankName, note, isHidden`

`last4` maps BeeCount `cardLastFour`. `openingBalanceCents` retains LifeTrace's integer-money convention rather than BeeCount `double initialBalance`.

### Category

Retain `parentId`; add:

`ledgerId, sortOrder, level, iconType, customIconFileId`

The server does not persist a device-local file path. Custom icons refer to a LifeTrace file/entity ID when synchronized.

### Transaction

Retain all current transaction fields and add:

`ledgerId, recurringTransactionId, excludeFromStats, excludeFromBudget, nativeAmountCents, nativeCurrency, exchangeRate, tag summary is NOT embedded`

Tags are normalized through `finance.transaction_tag`. For transfers, the existing `transactionType=transfer`, `accountId`, `toAccountId` model is retained instead of creating a second transfer record type.

Money stays integer cents on the wire. `nativeAmountCents` is the amount snapshot converted to the ledger base currency; account-local `amountCents` remains in `currency`.

### Recurring transaction

`meta, ledgerId, transactionType, amountCents, currency, categoryId, accountId, toAccountId, note, frequency, interval, dayOfMonth, dayOfWeek, monthOfYear, startDate, endDate, lastGeneratedDate, enabled`

### Tag

`meta, ledgerId, name, color, sortOrder, isArchived`

### Transaction tag

`meta, transactionId, tagId`

### Budget

`meta, ledgerId, budgetType(total|category), categoryId, amountCents, currency, period(monthly|weekly|yearly), startDay, enabled`

### Transaction attachment

`meta, transactionId, fileName, originalName, fileSize, width, height, sortOrder, fileId, sha256`

This entity stores metadata only. Binary transport continues to use LifeTrace's existing file subsystem when available; no separate object-storage service is introduced here.

## 5. Compatibility and migration rules

1. Existing `finance.account/category/transaction/transaction_evidence` schema-v1 payloads must remain readable during rollout.
2. New optional fields use sensible defaults so old data can be pulled by the new Android app.
3. New typed entity descriptors are registered in `lifetrace-contracts::registry` and dispatched through `EntityPayload`.
4. No new PostgreSQL business tables are required for these entities: the existing `sync_entities`/`sync_change_log` repository remains the source of sync persistence.
5. Relationship dependencies are supplied in `SyncChangeV1.dependencies` where appropriate (ledger/account/category/transaction/tag), enabling the existing server dependency check.
6. Android local profile IDs never replace cloud `meta.userId`; ownership still comes from authenticated principal.
7. Deletions use existing tombstone semantics and server versions.

## 6. Implementation order

1. Extend `lifetrace-contracts` finance DTOs, entity constants, registry descriptors, payload dispatch and tests.
2. Extend finance OpenAPI/JSON-schema exports only where generated/static contract artifacts require it.
3. Keep `services/cloud` generic sync repository unchanged except for finance route exposure/tests where useful.
4. Expand Android Room schema to v2 with a non-destructive v1->v2 migration.
5. Expand Android DAO/repository and `SyncEngine.RemoteMapper` for all new entity types.
6. Expand `LifeTraceContract.FINANCE_ENTITY_TYPES` so pull/snapshot includes the new entities.
7. Add contract/migration/sync tests in both repositories.
8. Run LifeTrace Rust/contract tests and Android/core tests in CI.
9. Update this document with completion/test results before any merge to `main`.

## 7. Acceptance criteria

- Existing v1 Android database upgrades without data loss.
- Existing LifeTrace finance data remains readable and syncable.
- A ledger, account, nested category, transaction, transfer, recurring template, tag relation, budget and attachment metadata can round-trip through the existing LifeTrace sync service.
- `excludeFromStats` changes statistics queries but not account balance semantics.
- `excludeFromBudget` changes budget usage independently of `excludeFromStats`.
- Hidden accounts remain part of balance/net-worth data but are excluded from normal account pickers.
- No new cloud deployment/service is created.
- CI/tests pass on both feature branches before merge.
