# LifeTrace EPIC-02 当前契约审计报告

> 审计时间：2026-08-05
> 审计对象：仓库 `D:\WorkSpace\LifeTrace`（Git 基线 `zhouxingxing1279/LifeTrace` main，`b903aa1`，版本 0.2.1）
> 审计方式：只读。仅阅读源码与文档，未执行任何写入、未修改任何领域类型。

---

## 0. 阅读范围

按 EPIC-02 前置要求完整阅读：

- `README.md`
- `package.json`
- `src/types/index.ts`、`src/types/english.ts`
- `src-tauri/Cargo.toml`
- `src-tauri/src/lib.rs`
- `src-tauri/src/server.rs`
- `src-tauri/src/server/state.rs`
- `src-tauri/src/server/migration.rs`
- `src-tauri/src/server/notes.rs`
- `src-tauri/src/server/english.rs`
- `src-tauri/src/server/imports.rs`
- `src-tauri/src/server/xunji.rs`
- `src-tauri/src/database/**`（migration runner、6 个 migration、repositories、legacy、backup、validation、connection）
- `docs/epic-01/**`（current-schema-audit、target-schema、migration-guide、rollback-guide、validation-report）
- 补充阅读：`src/stores/useLifeStore.ts`、`src/db/sqliteClient.ts`、`src/services/noteApi.ts`、`src/utils/finance.ts`、`src/utils/id.ts`、`src/components/HengXuShell.tsx`（备份 DTO 片段）、`src-tauri/src/server/assistant.rs`、`src-tauri/src/server/translation.rs`、`src-tauri/src/server/photo.rs`、`src-tauri/src/server/desktop.rs`、`src-tauri/src/database/backup.rs`、`docs/LifeTrace_EPIC02_Agent_Implementation_Plan.md`

全仓库搜索（排除 node_modules / .git / build / dist / target）：
`interface`、`type`、`struct`、`enum`、`serde`、`data_json`、`amount`、`amount_cents`、`createdAt`、`updatedAt`、`version`、`device`、`sync`、`outbox`、`cursor`、`change_id`、`schema_version`、`backup`

关键结论：**当前仓库没有任何 sync outbox、change log、cursor、change_id、schema_version、serverVersion 的生产实现**，这些词仅出现在 `LifeTrace_Future_Requirements.md`、`docs/LifeTrace_Complete_Roadmap_v3.md` 和 EPIC-02 方案文档中。

---

## 1. 当前所有 TypeScript 领域类型

### 1.1 `src/types/index.ts`（核心领域）

| 类型 | 说明 | 关键字段差异 |
|---|---|---|
| `ActivityType` / `ActivityColorKey` / `ActivityScheduleType` / `ActivityCheckinMethod` / `ActivitySyncSource` | 字符串字面量联合 | 与 DB CHECK 一致 |
| `Activity` | 习惯项目 | `id` 允许历史非 UUID（如 `piano`） |
| `ActivityLog` | 打卡记录 | `status` 联合 `completed/partial/skipped`；`metadata` 内嵌 `{state, urgeLevel, triggers, actions}` |
| `Transaction` | 财务交易 | `amount: number`（元，浮点）；`type` 仅 3 值；`category`/`account` 为展示字符串 |
| `FinanceAccount` | 账户 | `balance: number \| null`（元）；无 `currency` 字段 |
| `WorkoutHistorySet` / `WorkoutHistoryExercise` / `WorkoutHistory` | 训练记录（嵌套结构） | 嵌套 sets/exercises，与 DB 规范化 3 表不同 |
| `XunjiWorkoutSet` / `XunjiWorkoutExercise` / `XunjiWorkout` | 训记解析中间结构 | 仅 `weightKg/reps/setNumber` |
| `WorkoutImportRecord` | 训练导入记录 | `rawData: unknown`；`workout?: XunjiWorkout` |
| `TrainingNote` | 训练笔记 | `workoutRecordId`（对应 DB `workout_id`） |
| `DailyReview` | 每日复盘 | `reviewDate: string`（YYYY-MM-DD） |
| `NoteType` | 笔记类型联合 | 8 值 |
| `NoteFolder` / `NoteTag` | 文件夹/标签 | 无 `userId` |
| `NoteRelation` | 跨实体关联 | `entityType` 联合仅 7 值（habit/habit_checkin/workout/exercise/transaction/account/project） |
| `NoteAttachment` | 附件元数据 | `storagePath` 指向本地文件 |
| `NoteRevision` | 笔记版本 | `version: number` |
| `Note` | 笔记 | `version: number`；嵌套 `tags/relations/attachments` |
| `ViewId` | 视图枚举 | UI 层 |

### 1.2 `src/types/english.ts`（每日英语）

| 类型 | 说明 |
|---|---|
| `CEFRLevel` / `EnglishCategory` / `EnglishProcessingStatus` / `EnglishFetchStatus` / `EnglishSourceStatus` / `EnglishSyncTaskStatus` | 字符串字面量联合 |
| `ArticleVocabularyItem` | 文章内嵌词汇 |
| `EnglishArticle` | 文章（**无 userId**，全局目录；含 `content` 全文、`vocabulary`、`questions`） |
| `EnglishSourceSyncResult` / `EnglishContentSourceState` / `EnglishSyncTask` / `EnglishSyncLog` | 抓取/同步运维模型（设备本地） |
| `EnglishLibraryStats` | 统计视图模型 |
| `EnglishCompletionStatus` / `EnglishReadingStatus` | 学习状态 |
| `EnglishLearningRecord` | 学习记录 |
| `EnglishVocabulary` | 生词（旧版精简 DTO） |
| `VocabularyStatus` / `VocabularyReviewResult` | 生词状态（大写枚举） |
| `DictionaryLookup` / `UserVocabulary` / `VocabularyOccurrence` / `VocabularyReviewLog` | 词典与生词工作区 |
| `VocabularySettings` | 发音/复习设置（本地） |
| `EnglishTextAnchor` / `EnglishHighlight` / `EnglishNote` | 阅读标注 |
| `EnglishMistake` / `EnglishAIAnalysis` | AI 分析（provider: mock/deepseek） |
| `EnglishTodayResponse` / `EnglishHistoryResponse` | 页面响应聚合 |

### 1.3 支撑/UI 层类型（非领域）

`LifeData`、`LifeSettings`、`SQLiteMutation`（`src/db/sqliteClient.ts`）；`AccountBalanceSnapshot`（`src/utils/finance.ts`）；`NoteInput`/`ListOptions`（`src/services/noteApi.ts`）；组件内联类型（PhotoSyncModule 的 `Photo/Device/UploadTask/Dashboard`、HengXuShell 的 `Modal/ImportUploadItem` 等）。

---

## 2. 当前所有 Rust 领域表示

### 2.1 Repository Row（`src-tauri/src/database/repositories/`）

| 结构体 | 位置 | 说明 |
|---|---|---|
| `AccountRow` | finance.rs:80 | 规范化账户行，snake_case，`String` 字段为主 |
| `TransactionRow` | finance.rs:100 | 规范化交易行，含 `amount_cents: i64` |
| `ActivityRow` / `ActivityLogRow` / `DailyReviewRow` | habits.rs:43/70/88 | 习惯与复盘行 |
| `WorkoutRow` / `WorkoutExerciseRow` / `WorkoutSetRow` | workouts.rs:48/38/30 | 训练三表行 |
| `WorkoutImportRow` | workouts.rs:336 | 导入记录行 |
| `TrainingNoteRow` | workouts.rs:497 | 训练笔记行 |

notes 与 english 仓库没有独立 Row struct，直接构造 `serde_json::json!({...})` 返回 camelCase DTO；notes 读取时对 `content_json` 做 `serde_json::from_str` 解析。

### 2.2 服务端请求/响应 struct

| 结构体 | 位置 | 说明 |
|---|---|---|
| `Health` | server.rs:34 | `rename_all = "camelCase"` |
| `ImportUpload` | imports.rs:19 | `rename_all = "camelCase"`，JSON 表 |
| `WorkoutSet` / `WorkoutExercise` / `Workout` / `ImportAction` | xunji.rs:25/33/39/53 | `rename_all = "camelCase"` |
| `NoteQuery` | notes.rs:21 | `rename_all = "camelCase"`（query 参数） |
| `SettingsInput` / `ChatInput` / `ConversationInput` | assistant.rs:176/188/199 | `rename_all = "camelCase"`；`SettingsInput` 含 `api_key`（secret） |
| `SettingsInput` / `TranslateInput` | translation.rs:16/22 | `rename_all = "camelCase"`；含 `secret` |
| `LookupQuery` | dictionary.rs:20 | 词典查询 |
| `DesktopState` | desktop.rs:14 | 桌面状态 |
| `Runtime` | photo.rs:34 | 照片运行时（证书/密钥路径） |

### 2.3 数据库/备份结构

`MigrationContext`、`MigrationReport`、`MigrationError`、`AppliedMigration`、`MigrationSummary`（migration_runner.rs）；`BackupRecord`（backup.rs:13）。

### 2.4 结论

- **没有独立的 Domain Model 层**：Rust 在 Row struct 与 `serde_json::Value` DTO 之间直接转换，无实体注册表、无统一 `EntityMeta`。
- **没有 Wire DTO 权威定义**：DTO 散落在各 repository 的 `json!` 构造中。
- **没有错误码枚举**：错误以 `{ "error": string }`（state/notes/imports/xunji）或 `{ "error", "code" }`（assistant/translation/photo）返回，code 为任意字符串，无稳定枚举。

---

## 3. EPIC-01 数据表和字段

EPIC-01 已通过 6 个版本化 Migration 落地（`schema_migrations` 记录到 version 6），并保留旧 JSON 表为 `legacy_*_json_v1`。

### 3.1 规范化业务表

| 表 | 关键字段 | 说明 |
|---|---|---|
| `finance_accounts` | id, user_id, name, account_type, **opening_balance_cents**, balance_at, last4, color, icon, is_archived, created_at, updated_at, deleted_at, **version**, modified_by_device | 金额以整数分存储 |
| `transaction_categories` | id, user_id, name, category_type(expense/income/transfer), parent_id, icon, color, is_system, is_archived, 公共字段 | 唯一 `(user_id, category_type, name)` |
| `transactions` | id, user_id, transaction_type(expense/income/transfer/refund/fee), **amount_cents**, currency(默认 CNY), account_id, to_account_id, category_id, counterparty, merchant, item, note, occurred_at, **local_date**, status(candidate/provisional/confirmed/ignored), source_type, external_transaction_id, legacy_category_name, legacy_account_name, raw_json, 公共字段 | 金额 CHECK >= 0；外键到账户/分类 |
| `transaction_evidence` | id, transaction_id(FK CASCADE), source_type, source_id, external_transaction_id, confidence, raw_json, created_at | 无 user_id/version |
| `activities` | id, user_id, name, activity_type, unit, minimum_target, normal_target, target_period, target_days_json, icon, color, schedule_type, start_date, checkin_method, sync_source, description, is_archived, 公共字段 | |
| `activity_logs` | id, user_id, activity_id(FK), log_date, value, status, note, metadata_json, 公共字段 | |
| `daily_reviews` | id, user_id, review_date, energy(1-10), mood(1-10), completion_score, best_thing, problem, tomorrow_priority, note, 公共字段 | 唯一 `(user_id, review_date)` |
| `note_folders` / `note_tags` | id, user_id, name, icon/color, sort_order(仅文件夹), 公共字段 | 标签唯一 `(user_id, name)` |
| `notes` | id, user_id, title, note_type, folder_id(FK), content_json, content_html, content_text, content_markdown, summary, is_pinned, is_favorite, is_archived, ai_summary, ai_tags_json, embedding_status, last_ai_processed_at, 公共字段 | FTS5：`notes_fts` |
| `note_tag_relations` | note_id, tag_id, created_at（复合主键） | 无 user_id/version |
| `note_relations` | id, note_id(FK CASCADE), entity_type, entity_id, relation_type, created_at | 无 user_id/version |
| `note_attachments` | id, note_id(FK CASCADE), file_name, original_name, mime_type, file_size, storage_path, created_at | 无 user_id/version；文件本体在磁盘 |
| `note_revisions` | id, note_id(FK CASCADE), revision_version, title, content_json, content_html, content_markdown, created_at | 唯一 `(note_id, revision_version)` |
| `english_articles` | id, title, level, category, content, word_count, difficulty, estimated_minutes, source*, published_at, content_hash, quality_score, questions_json, vocabulary_json, raw_json, created_at, updated_at, deleted_at, version, modified_by_device | **无 user_id（全局目录）** |
| `english_learning_records` | id, user_id, article_id(FK), record_date, reading_time_seconds, summary, score, analysis_id, new_words_json, completion_status, reading_status, started_at, completed_at, 公共字段 | |
| `english_highlights` / `english_notes` | id, user_id, article_id(FK), 标注字段（block_id/start_offset/end_offset/prefix/suffix 等）, 公共字段 | |
| `english_vocabulary` | id, user_id, normalized_word, display_word, definition, phonetic, part_of_speech, selected_meanings_json, lemma, source_article_id, notes, mastery_level, review_stage, review_count, correct_count, incorrect_count, encounter_count, last_reviewed_at, next_review_at, status, frequency_rank, tags_json, metadata_json, 公共字段 | 唯一 `(user_id, normalized_word)` |
| `vocabulary_occurrences` | id, vocabulary_id(FK CASCADE), article_id, article_title, source_sentence, created_at | 无 user_id/version |
| `vocabulary_review_state` | vocabulary_id(PK FK CASCADE), due_at, difficulty, stability, retrievability, review_count, lapse_count, scheduler_version, updated_at | FSRS 预留 |
| `english_ai_analysis` | id, user_id, record_id(FK), article_id, provider, score, content_score, grammar_score, vocabulary_score, structure_score, mistakes_json, suggestions_json, improved_summary, weak_points_json, 公共字段 | 不在 EPIC-02 实体清单内 |
| `workouts` | id, user_id, source, source_id, name, occurred_at, local_date, duration_seconds, exercise_count, set_count, planned_set_count, volume_kg, calories_kcal, status, raw_json, 公共字段 | |
| `workout_exercises` | id, workout_id(FK CASCADE), name, sort_order, planned_sets, completed_sets | 无 user_id/version |
| `workout_sets` | id, exercise_id(FK CASCADE), set_number, weight_kg, reps, completed | 无 user_id/version |
| `workout_imports` | id, user_id, source, share_url, status, parser, parser_version, error, raw_json, workout_id(FK), 公共字段 | |
| `training_notes` | id, user_id, title, content, workout_id(FK), source, note_date, 公共字段 | |

公共字段 = `created_at, updated_at, deleted_at, version INTEGER DEFAULT 1, modified_by_device`。

### 3.2 保留的 KV / 配置 / 运维表

| 表 | 存储位置 | 说明 |
|---|---|---|
| `settings` | state.rs | `(id, data_json, updated_at)`，`preferences` |
| `import_uploads` | imports.rs | `(id, data_json, updated_at)`，文件本体在 `data_dir/imports/` |
| `ai_settings` / `ai_conversations` | assistant.rs | `ai_settings` 含 **apiKey** |
| `translation_settings` | translation.rs | 含 **secret** |
| `english_preferences` / `english_sources` / `english_sync_tasks` | english.rs | 抓取/运维配置 |
| `photos` / `photo_sync_devices` / `photo_upload_tasks` / `photo_device_assets` | photo.rs | 照片同步；`photo_sync_devices` 含 **token_hash** |
| `app_meta` / `schema_migrations` / `migration_runs` / `migration_issues` | migration_runner.rs | 迁移元数据 |

---

## 4. API DTO

### 4.1 本地 HTTP API（127.0.0.1:3103）

| 端点 | 请求 | 响应 | 命名 |
|---|---|---|---|
| `GET /api/health` | - | `{ok, runtime, dataDir}` | camelCase |
| `GET /api/state` | - | `{activities, logs, reviews, workoutHistory, settings, accounts, transactions}` | camelCase |
| `POST /api/state` | `{operation: put/patch/delete/restore, table, value/id/patch/data}` | `{ok:true}` 或 `{error}` | camelCase |
| `GET/POST /api/notes` | `{action: create/update/trash/restore/delete/duplicate/folder.save/...}` | Note / `{ok,id}` / `{error}` | camelCase |
| `/api/english/*` | 多种 | camelCase DTO（Article/Record/Highlight/Note/Vocabulary/Analysis/Settings 等） | camelCase |
| `POST /api/xunji/parse` / `GET/POST /api/xunji/imports` | `{importId, action, workout}` | XunjiWorkout / import 列表 | camelCase |
| `/api/imports` | multipart | `ImportUpload{id,kind,filename,contentType,size,status,objectKey,createdAt,updatedAt}` | camelCase |
| `/api/settings/ai` | `{apiKey, model}` | `{configured, model, error}` | camelCase，**含密钥** |
| `/api/settings/translation` | `{appId, secret}` | `{configured, error}` | camelCase，**含密钥** |
| `/api/photo-sync/*` | 见 photo.rs | Dashboard/Pairing/Task | camelCase |
| `/api/assistant/*` | `{messages}`, conversations | ChatResponse 等 | camelCase |

### 4.2 错误 DTO

- state/notes/imports/xunji：`{"error": string}`（无 code）
- assistant/translation/photo：`{"error": string, "code": string}`（code 为自由字符串，如 `DEVICE_TOKEN_INVALID`、`AI_NOT_CONFIGURED`）

**没有统一 ErrorCode 枚举、没有 requestId、没有 retryable/fieldErrors/details。**

---

## 5. 备份 DTO

当前存在三套互相独立的备份格式，**均不是同步契约**：

### 5.1 前端 JSON 备份（HengXuShell.tsx:408）

```json
{
  "format": "lifetrace-backup",
  "schemaVersion": 2,
  "createdAt": "<RFC3339>",
  "activities": [], "logs": [], "transactions": [],
  "reviews": [], "accounts": [], "workoutHistory": []
}
```

### 5.2 笔记 JSON 备份（notes.rs `backup()`）

```json
{
  "format": "lifetrace-notes",
  "version": 2,
  "createdAt": "<RFC3339>",
  "notes": [], "folders": [], "tags": [], "revisions": []
}
```

### 5.3 SQLite 一致性备份（backup.rs）

- 文件：`{data_dir}/backups/database/lifetrace-before-schema-v*.db`（SQLite Backup API 快照，含 `integrity_check` + SHA-256）
- `BackupRecord { path, sha256, size_bytes, integrity_ok }`，保留最近 3 份

---

## 6. camelCase 与 snake_case 差异

| 层 | 命名 | 示例 |
|---|---|---|
| SQLite 列 | snake_case | `amount_cents`、`created_at`、`modified_by_device` |
| Wire JSON（API 响应/请求） | camelCase | `amountCents`（仅 Rust 内部）、`createdAt`、`modifiedByDevice`（DTO 未暴露） |
| 前端 TS 领域类型 | camelCase | `amount`、`createdAt`、`workoutRecordId` |
| Rust Row struct | snake_case | `amount_cents`、`user_id` |
| Rust 服务端请求 struct | 混用 | 多数 `rename_all = "camelCase"`；`ChatInput`/`TranslateInput`/`ConversationQuery` 无 rename（字段本身无下划线） |
| Rust 迁移代码 | snake_case | 与列一致 |

差异点：
- 前端 DTO 中 `category`/`account`（展示名）↔ DB `category_id`/`account_id` + `legacy_category_name`/`legacy_account_name`。
- 前端 `WorkoutHistory.templateId/sourceId/workoutRecordId` ↔ DB `workouts.source_id`、`training_notes.workout_id`。
- 前端 `NoteAttachment.fileName/originalName/storagePath` ↔ DB `file_name/original_name/storage_path`。
- EnglishArticle `createdTime` ↔ DB `created_time`；`sourceKey` ↔ `source_key`。

---

## 7. 金额表示差异

| 位置 | 表示 | 示例 |
|---|---|---|
| 前端 TS `Transaction.amount` | `number`（元，浮点） | `4`、`125.25` |
| 前端 TS `FinanceAccount.balance` | `number \| null`（元） | `128.31` |
| DB `transactions.amount_cents` | `INTEGER`（分） | `12525` |
| DB `finance_accounts.opening_balance_cents` | `INTEGER \| NULL`（分） | `12831` |
| Rust 内部 | `i64` 分；DTO 转换 `cents_to_amount`（÷100 → f64） | finance.rs:38 |
| 前端余额计算 | `Math.round(value*100)` 转分后累加 | utils/finance.ts:6 |

问题：
- 前端金额是浮点，存在精度风险；DB 已规范化，但**线路上没有权威的 `amountCents` 字段**（`/api/state` 返回的仍是 `amount`）。
- **`currency` 在 DB 有默认 CNY，但前端 `Transaction`/`FinanceAccount` 类型完全没有 currency 字段**。
- 账户只有 `opening_balance_cents`，前端 `balance` 为“基准余额”，无 `balanceCents` 权威字段。

---

## 8. 时间表示差异

| 语义 | 前端 TS | Rust/DB | 问题 |
|---|---|---|---|
| 时间点 | `new Date().toISOString()`（RFC3339 UTC，带毫秒 `000Z`） | `chrono::Utc::now().to_rfc3339()`（RFC3339，可带 `+00:00`） | 格式不完全统一（`Z` vs `+00:00`、毫秒有无） |
| 自然日 | `reviewDate: "2026-07-22"`、`noteDate`、`startDate` | `review_date`/`note_date` 存 `YYYY-MM-DD`；`local_date` 由 RFC3339 前 10 字符推导 | **local_date 按 UTC 截取，不等于设备本地自然日** |
| 时间戳比较 | `Date.parse` | 字符串比较 | 依赖规范化后的字典序 |
| 排序 | 前端按 `createdAt`/`updatedAt` 排序 | 后端 `ORDER BY updated_at DESC` | 无全局顺序权威 |

EPIC-02 需要：Wire 时间点统一 RFC3339 UTC；自然日显式 `YYYY-MM-DD` 字段；`clientModifiedAt` 只作审计。

---

## 9. ID 类型差异

| 类型 | 前端 | Rust | 现状 |
|---|---|---|---|
| 实体 ID | `string`（`src/utils/id.ts` 生成 UUID v4，回退 `local-<ts>-<rand>`） | `String`（Row）；新 ID `Uuid::new_v4().to_string()` | 历史非 UUID ID 大量存在：`piano`、`wechat-wallet`、`xunji-...`、`training-note-xunji-...` |
| userId | `"local-user"` 硬编码 | `DEFAULT_USER_ID = "local"` | **前端与 DB 默认值不一致**（`local-user` vs `local`），Repository 在保存时兜底为 `local` |
| deviceId | 无领域类型 | 照片模块 `device_uuid`/`device_id` | 无统一 DeviceId 类型 |
| 版本/游标 | 无 | 无 | 无 serverVersion/cursor 概念 |

结论：EPIC-02 的 `EntityId`/`UserId`/`DeviceId` 必须是**字符串 newtype**（兼容非 UUID 历史 ID），不能强制 `Uuid`；新 ID 继续 UUID v4。

---

## 10. nullable 差异

| 场景 | 前端 | DB | 差异 |
|---|---|---|---|
| `Note.title` | `string \| null` | `TEXT`（可空） | 一致 |
| `Note.deletedAt` | `string \| null` | `deleted_at TEXT` | 一致 |
| `FinanceAccount.balance` | `number \| null`（必填属性可空） | `opening_balance_cents INTEGER`（可空） | 一致 |
| `Activity.minimumTarget` 等 | `?`（可选 → 可缺失） | `minimum_target REAL`（可空列） | 前端可能缺失字段，DB 可能返回 `null` |
| `Transaction.category/account` | 必填 string | DB 只有 `category_id`/`legacy_category_name` | 结构不一致 |
| `ActivityLog.status` | `?` | `status TEXT` 可空 | 前端可缺失 |
| `NoteTag`/`NoteFolder` | 无 userId 字段 | 有 `user_id`（默认 local） | 前端不暴露 |
| `modified_by_device` | 前端无 | 多数主表有列 | 前端 DTO 不暴露 |
| `version` | 仅 Note 暴露 `version: number` | 多数主表有 `version INTEGER` | 其余实体不暴露 |

---

## 11. 枚举差异

| 概念 | 前端 TS | DB CHECK / Rust 校验 | 差异 |
|---|---|---|---|
| 交易类型 | `"expense" \| "income" \| "transfer"` | `expense/income/transfer/refund/fee`（5 值） | **前端缺 refund/fee** |
| 交易状态 | 无 | `candidate/provisional/confirmed/ignored` | 前端无 |
| 账户类型 | `cash/bank/wechat/alipay/investment/other` | 同 | 一致 |
| 习惯类型 | 5 值 | 同 | 一致 |
| 打卡状态 | `completed/partial/skipped` | 同 | 一致 |
| 复盘评分 | 无约束 | `energy/mood BETWEEN 1 AND 10` | 前端无校验 |
| 笔记类型 | 8 值 | `note_type` 无 CHECK | DB 不校验 |
| 英语处理状态 | `FETCHED/CLEANED/ANALYZED/READY/REJECTED/FAILED` | 无 CHECK | DB 不校验 |
| 生词状态 | `LEARNING/REVIEWING/MASTERED/ARCHIVED` | `status TEXT` 默认 LEARNING | 无 CHECK |
| 关系类型 | `reference/created_from/summary/attachment` | `relation_type` 无 CHECK | DB 不校验 |
| 错误码 | 无 | 自由字符串 | 无稳定枚举 |

Rust 侧全部为 `String` + 常量数组校验，**没有 enum**（仓库内无 `enum` 声明，仅字符串常量数组）。

---

## 12. 哪些实体需要同步

### 12.1 双向同步（user_owned）

| entity type | DB 表 | 说明 |
|---|---|---|
| finance.account | finance_accounts | 金额权威化 |
| finance.category | transaction_categories | 前端尚无类型 |
| finance.transaction | transactions | 核心 |
| finance.transaction_evidence | transaction_evidence | 关联 transaction |
| habit.activity | activities | |
| habit.log | activity_logs | |
| review.daily | daily_reviews | |
| note.folder | note_folders | |
| note.note | notes | 正文快照较大 |
| note.tag | note_tags | |
| note.tag_relation | note_tag_relations | |
| note.relation | note_relations | |
| note.revision | note_revisions | 体量大，需独立确认 |
| english.learning_record | english_learning_records | |
| english.highlight | english_highlights | |
| english.note | english_notes | |
| english.vocabulary | english_vocabulary | |
| english.vocabulary_occurrence | vocabulary_occurrences | |
| english.vocabulary_review_state | vocabulary_review_state | |
| workout.import | workout_imports | |
| workout.workout | workouts | |
| workout.exercise | workout_exercises | |
| workout.set | workout_sets | |
| workout.training_note | training_notes | |
| file.metadata | （尚无表，映射 note_attachments/照片） | 只同步元数据 |
| entity.link | note_relations（现有实现） | 统一跨实体关联 |
| user.preference | settings（preferences）/english_preferences 的子集 | 设备偏好是否同步需产品决策 |

### 12.2 服务端→客户端（server_to_client / shared_catalog）

| entity type | DB 表 | 说明 |
|---|---|---|
| english.article | english_articles | 无 user_id；内容可能很大（content/questions/vocabulary JSON） |

### 12.3 服务端管理（server_managed）

| entity type | 说明 |
|---|---|
| identity.user | 本 Epic 只定义 DTO；EPIC-04 注册/登录后由服务端管理 |
| identity.device | 本 Epic 只定义 DTO；EPIC-04 设备注册 |

---

## 13. 哪些实体由服务端管理

- `identity.user`、`identity.device`：服务端权威（EPIC-04 实现认证与注册）。
- `english.article`：内容目录由服务端/抓取管线管理（server_to_client，客户端只读）。
- `transaction_categories` 的 `is_system=1` 系统分类：可视为服务端管理种子数据（当前无系统分类写入逻辑，仅预留列）。

---

## 14. 哪些实体只在设备本地

| 实体/数据 | 表/位置 | 原因 |
|---|---|---|
| 照片与媒体 | photos、photo_device_assets、缩略图/原图文件 | 属于 EPIC-12 对象存储；本机文件 |
| 照片设备与配对 | photo_sync_devices、photo_upload_tasks、pairing 码 | 局域网配对状态，含 token_hash |
| 导入上传文件 | import_uploads + `data_dir/imports/` | 原始账单/训练文件，可能含隐私 |
| AI 会话与日志 | ai_conversations、english_sync_tasks、english_sync_logs | 运维/会话数据 |
| 英语内容源状态 | english_sources、english_preferences | 抓取调度配置（含 syncCursor 等） |
| 本地 TLS 证书 | `.local-certificates/`（server-cert.pem、server-key.pem） | 证书私钥永不离开设备 |
| 词典库 | xunji_service/data/dictionary.db | 随安装包分发的离线数据 |
| 迁移/备份元数据 | app_meta、schema_migrations、migration_runs、migration_issues、backups/ | 运维数据 |
| 前端 View Model | 组件内状态、zustand store | 不进入契约 |

---

## 15. 哪些凭据永远不得同步

| 凭据 | 位置 | 说明 |
|---|---|---|
| AI API Key | `ai_settings.data_json.apiKey`（DeepSeek） | 必须 `secret_local_only` |
| 翻译 Secret | `translation_settings.data_json.secret`（百度 appSecret） | 必须 `secret_local_only` |
| 照片设备 Token | `photo_sync_devices.token_hash`（SHA-256 哈希） | 即使已哈希也不进 Sync Payload |
| 配对码/临时令牌 | photo 配对流程内存/响应 | 不进 Sync Payload |
| 本地证书私钥 | `.local-certificates/server-key.pem` | 永不离开设备 |
| Refresh Token / 邮箱授权码 | 当前仓库无，但未来 EPIC-04 必须标记 | 按 EPIC-02 规则为 `secret_local_only` |
| 导入原始文件 | `import_uploads.raw_json`、`transactions.raw_json`、`workout_imports.raw_json` | 可能嵌入第三方原始数据/敏感信息；`raw_json` 字段不进同步 payload |

---

## 16. 对 EPIC-02 方案必须调整的地方

### 16.1 实体清单调整

1. **前端缺 `finance.category` 类型**：DB 已有 `transaction_categories`，必须新增 `TransactionCategory` DTO；`Transaction` 需同时携带 `categoryId` 与展示名兼容字段。
2. **`note.attachment` 不在规定清单**：附件二进制由 EPIC-12 处理；`note_attachments` 元数据映射到 `file.metadata` + `entity.link`，`note.note` 的 `containsFileReferences=true` 只引用文件 ID。
3. **`english.ai_analysis` 不在规定清单**：按用户清单不作为同步实体；建议 `device_local`（或未来新增 entity type，须升 schemaVersion 而非 protocolVersion）。
4. **`WorkoutHistory` 是嵌套 DTO，DB 是 3 张表**：Wire DTO 必须按 `workout.workout` + `workout.exercise` + `workout.set` 展开，不能把嵌套结构直接作为 payload。
5. **`english_articles` 无 user_id**：作为 `shared_catalog` + `server_to_client`；大 payload（全文、questions、vocabulary JSON）需要考虑 `maximumRequestBytes`/分页，建议轻量 DTO 携带 `contentHash`。
6. **`note_relations.entity_type` 现仅 7 值**：契约的 `EntityRef.entityType` 必须使用注册表实体类型（如 `finance.transaction`），并保留 `relationType` 可扩展字符串；不能硬编码表名。

### 16.2 ID 调整

7. **方案中 `EntityId(pub Uuid)` 必须改为字符串 newtype**：历史 ID（`piano`、`wechat-wallet`、`xunji-*`）非 UUID，同步时不得静默改写；新 ID 仍用 UUID v4。`UserId` 同样必须兼容 `local-user`/`local`。
8. **`userId` 前后端默认值不一致**（`local-user` vs `local`）：契约统一 UserId 字符串，桌面适配示例必须显式映射。

### 16.3 金额/时间调整

9. **金额**：Wire 必须 `amountCents: i64` + `currency`；前端 `Transaction.amount`（元浮点）是 UI View Model，不再作为 Wire 字段。账户余额对应 `openingBalanceCents`。
10. **时间**：`local_date` 现按 UTC 前 10 字符推导，不等于设备本地日；Wire 必须显式 `localDate: YYYY-MM-DD`，`occurredAt` 为 RFC3339 UTC，不得互推。
11. **版本**：DB `version INTEGER`（本地修订）不得充当 serverVersion；契约统一 `localVersion` / `serverVersion: Option<ServerVersion>` / `baseServerVersion`（字符串），离线修改不得伪造 serverVersion。

### 16.4 枚举调整

12. **交易类型前端 3 值 vs DB 5 值**（refund/fee）：契约枚举必须含全部 DB 值；同时使用 `Unknown(String)` 变体保证向前兼容。
13. **Rust 侧全部是 String + 常量数组**：契约改稳定枚举（serde 字符串），未知值落入 `Unknown(String)`。

### 16.5 架构调整

14. **当前 Rust 无 Domain Model 层**：契约 crate 建立独立 Domain DTO + Wire DTO；桌面 repository 负责 Row ↔ Domain ↔ Wire 转换，禁止把 `serde_json::Value` 直接当契约。
15. **错误模型不统一**：必须建立稳定 `ErrorCode` 枚举 + `ApiErrorV1{code, message, requestId, retryable, fieldErrors, details}`；HTTP 状态码规则按方案（200/400/401/403/413/426/429/500/503）。
16. **KV 配置表**（settings/ai_settings/translation_settings/english_preferences）属于 `secret_local_only` 或 `device_local`，不进入 Sync Payload；`user.preference` 只承载可同步的偏好子集。
17. **备份 DTO 与同步契约分离**：现有 `lifetrace-backup`/`lifetrace-notes` 格式是本地恢复格式，不得复用为 Snapshot 契约。
18. **photo 模块已自建“设备/Token/任务”模型**：与 identity.device 语义不同，EPIC-02 只定义 identity.device DTO，不把 photo_sync_devices 纳入同步。

### 16.6 实施环境调整

19. 仓库当前**无 `crates/`、`contracts/`、`tools/`、`docs/epic-02/` 目录**，阶段 1 起全部新建；需要新增 Cargo workspace 或独立 manifest（建议独立 manifest + 根目录 `Cargo.toml` workspace，避免影响 `src-tauri` 构建）。
20. `npm run lint` 的 tsconfig `include` 不含 `tests/`；生成的 TS 契约需通过 `src/types/contracts.ts` 桥接并纳入 include，保证 `contracts:check` 稳定。
21. 现有 `package.json` 无 contracts 脚本；需新增 `contracts:generate`、`contracts:test`、`contracts:check`（Windows 兼容：Rust 生成器、不依赖 bash/jq）。
22. **迁移状态确认**：`schema_migrations` 应已到 version 6，但 `settings`/`import_uploads` 等 JSON 表仍由 `ensure_schema` 创建（符合 EPIC-01 保留 KV 表的设计）；审计期间未打开真实数据库验证行数，阶段 1 开始时建议跑一次 `cargo test` 确认基线。

---

## 附录 A：EPIC-01 状态确认

- 6 个 Migration（framework / finance / habits-reviews / notes / english / workouts）已合入 main；README 已更新为规范化模型说明。
- 仓库仍有遗留 JSON 表创建逻辑（`settings`、`import_uploads`），符合 EPIC-01 “KV/配置表保留 JSON” 的目标结构。
- `docs/epic-01/validation-report.md` 记录迁移校验通过（金额分毫一致、唯一约束、FK 无异常）。

## 附录 B：审计期间未做

- 未修改任何 TS/Rust 领域类型。
- 未创建同步服务、Worker、outbox、网络调用。
- 未打开真实用户数据库写入；仅阅读源码与文档。
