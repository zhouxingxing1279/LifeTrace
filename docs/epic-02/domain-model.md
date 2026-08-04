# LifeTrace EPIC-02 统一领域模型

> 版本：v1（`protocolVersion=1`、`schemaVersion=1`）
> 权威来源：`crates/lifetrace-contracts`（Rust 类型），生成物见 `contracts/`

---

## 1. 分层原则

EPIC-02 强制区分四层，禁止把某层直接当作另一层：

| 层 | 位置 | 说明 |
|---|---|---|
| Database Row | `src-tauri/src/database/repositories/*`（`*Row` 结构） | SQLite snake_case 行 |
| Domain Model / Wire DTO | `crates/lifetrace-contracts/src/domain/*` | 公共契约，wire camelCase |
| Sync Wire | `crates/lifetrace-contracts/src/sync/v1/*` | Push/Pull/Snapshot/Conflict/Tombstone/Capabilities |
| UI View Model | `src/types/*.ts`、组件内类型 | 前端展示模型，允许独立演进 |

转换路径：

```text
SQLite Row → Repository → Domain DTO → Sync DTO → JSON
```

桌面适配示例见 `src-tauri/src/contracts.rs`（财务交易）。

## 2. 基础值对象

| 类型 | wire 表示 | 说明 |
|---|---|---|
| `UserId` / `DeviceId` / `EntityId` / `ChangeId` / `RequestId` / `ConflictId` / `AtomicGroupId` / `SnapshotId` | string | 新 ID 使用 UUID v4；历史非 UUID ID（`piano`、`wechat-wallet`、`xunji-*`）原样保留，同步时不得改写 |
| `Cursor` | string（opaque） | 服务端生成，客户端不得加减/猜测/比较不同服务器 |
| `ServerVersion` | string（十进制，如 `"42"`） | 服务端权威实体版本，避免 JS 安全整数问题 |
| `MoneyAmount` | `{amountCents: integer, currency: string}` | 金额永远整数分，禁止 wire 浮点 |
| `CurrencyCode` | string（3 个大写字母） | 默认 `CNY` |
| `LocalDate` | `YYYY-MM-DD` | 自然日，不与 UTC 时间点互推 |
| `UtcTimestamp` | RFC3339 UTC | 时间点 |
| `EntityMeta` | 见下 | 实体公共元数据 |

## 3. EntityMeta

```json
{
  "id": "tx-1",
  "userId": "local-user",
  "createdAt": "2026-08-04T15:30:00Z",
  "updatedAt": "2026-08-04T15:30:00Z",
  "deletedAt": null,
  "localVersion": 3,
  "serverVersion": null,
  "modifiedByDevice": null
}
```

- `localVersion`：本地修订号（对应 EPIC-01 `version` 列），不是服务器权威版本。
- `serverVersion`：服务端分配；本地新实体为 `null`，`baseServerVersion` 用 `"0"`。
- 离线修改只增加 `localVersion`，**不得伪造 `serverVersion`**。

## 4. 领域 DTO 清单

### identity

- `identity.user`：`User { meta, displayName?, email?, status }`
- `identity.device`：`Device { meta, deviceName, platform, appId?, status, lastSeenAt? }`

### finance

- `finance.account`：`FinanceAccount { meta, name, accountType, openingBalanceCents?, balanceAt?, last4?, color, icon, isArchived, currency }`
- `finance.category`：`TransactionCategory { meta, name, categoryType, parentId?, icon?, color?, isSystem, isArchived }`
- `finance.transaction`：`Transaction { meta, transactionType, amountCents, currency, accountId?, toAccountId?, categoryId?, counterparty?, merchant?, item?, note?, occurredAt, localDate, status, sourceType, externalTransactionId? }`
- `finance.transaction_evidence`：`TransactionEvidence { meta, transactionId, sourceType, sourceId?, externalTransactionId?, confidence? }`

注意：`category`/`account` 展示名、`legacy_*` 字段、`raw_json` 不进 wire DTO。

### habit / review

- `habit.activity`：`Activity { meta, name, activityType, unit, minimumTarget?, normalTarget?, targetPeriod, targetDays[], icon?, color?, scheduleType?, startDate?, checkinMethod?, syncSource?, description?, isArchived }`
- `habit.log`：`ActivityLog { meta, activityId?, logDate, value?, status?, note?, metadata? }`
- `review.daily`：`DailyReview { meta, reviewDate, energy?, mood?, completionScore?, bestThing?, problem?, tomorrowPriority?, note? }`

### note

- `note.folder` / `note.tag` / `note.tag_relation`（复合 ID `<noteId>:<tagId>`）/ `note.relation` / `note.revision`
- `note.note`：`Note { meta, title?, noteType, folderId?, contentJson, contentHtml, contentText, contentMarkdown, summary, isPinned, isFavorite, isArchived, aiSummary?, aiTags?, embeddingStatus?, lastAiProcessedAt? }`
  - 同步 payload 必须是权威全文快照；列表 DTO 可省略正文（属 UI 层）。
  - tags/relations/attachments 由独立实体表达，不嵌套在 Note 里。

### english

- `english.article`（shared_catalog，无用户归属语义；含全文、questions、vocabulary）
- `english.learning_record` / `english.highlight` / `english.note`
- `english.vocabulary`（`normalizedWord` 唯一）/ `english.vocabulary_occurrence` / `english.vocabulary_review_state`
- `english.ai_analysis` **不在 v1 实体清单**，视为设备本地数据；如需同步需在后续版本新增 entity type。

### workout

- `workout.import` / `workout.workout` / `workout.exercise` / `workout.set` / `workout.training_note`
- 前端 `WorkoutHistory` 的嵌套 exercises/sets 展开为独立实体；`raw_json` 不进 wire。

### file / link / preference

- `file.metadata`：`{ meta, originalName, mimeType, sizeBytes, sha256, storageState, createdByDevice? }`——**不含**对象存储 key、预签名 URL、本地绝对路径。
- `entity.link`：`{ meta, source: EntityRef, target: EntityRef, relationType, metadata? }`
- `user.preference`：`{ meta, preferenceKey, value }`——秘密偏好不属于可同步子集。

## 5. 命名与格式约定

- wire JSON 一律 camelCase；SQLite 列 snake_case；Rust Row 结构 snake_case。
- 时间点 RFC3339 UTC；自然日 `YYYY-MM-DD`。
- 金额 `amountCents` 整数分；`localDate` 显式传递，禁止用 UTC 前 10 字符推导业务自然日。
- 未知 JSON 字段忽略；未知枚举值作为字符串保留。

## 6. 生成物

- JSON Schema：`contracts/json-schema/*.schema.json`（95 个）
- TypeScript：`contracts/typescript/lifetrace-contracts.generated.ts`（95 个类型，全部 `export`）
- OpenAPI：`contracts/openapi/lifetrace-sync-v1.json`（OpenAPI 3.1）

生成命令：`npm run contracts:generate`；校验：`npm run contracts:check`。
