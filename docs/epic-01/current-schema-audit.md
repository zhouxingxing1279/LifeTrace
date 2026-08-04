# LifeTrace EPIC-01 当前 Schema 审计报告

> 审计时间：2026-08-04
> 审计对象：仓库 `D:\WorkSpace\LifeTrace`（Git 基线 `zhouxingxing1279/LifeTrace` main，版本 0.2.1）
> 审计方式：只读阅读源码 + 只读打开真实数据库 + 只读检查旧备份

---

## 1. 审计范围与方法

本次审计只读完成了以下工作：

- 完整阅读 `src-tauri/src/server.rs`、`state.rs`、`migration.rs`、`notes.rs`、`english.rs`、`imports.rs`、`xunji.rs`
- 阅读 `src/types/index.ts`、`src/types/english.ts`、`src/stores/useLifeStore.ts`、`src/services/`（noteApi、pronunciation）
- 检索全仓库 `CREATE TABLE`、`ALTER TABLE`、`data_json`、`ensure_schema`、`Connection::open`、`INSERT INTO`、`UPDATE`、`DELETE FROM`、`/api/state`、备份/恢复逻辑
- 只读打开本机真实数据库 `%APPDATA%\com.lifetrace.desktop\lifetrace.db`（4.3MB，2026-08-03 最后修改）
- 只读检查仓库内旧备份：
  - `backups/lifetrace-before-clear-20260724-0221.sqlite`（旧桌面 JSON 库，1.2MB）
  - `backups/wrangler-state-before-action-library-cleanup-20260726/`（旧 D1 状态，含 8.7MB SQLite 业务库）

审计期间没有执行任何写入操作。

---

## 2. 当前 SQLite 表总览

### 2.1 表清单（按创建位置分组）

#### 核心状态表 —— 创建于 `src-tauri/src/server/state.rs::ensure_schema`（约 L96）

统一结构：

```sql
CREATE TABLE IF NOT EXISTS {table} (
  id TEXT PRIMARY KEY,
  data_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

| 表名 | 前端键名 | 真实库行数 | 旧备份行数 | 说明 |
|---|---|---|---|---|
| `activities` | `activities` | 2 | 5 | 习惯项目 |
| `activity_logs` | `logs` | 4 | 13 | 打卡记录 |
| `transactions` | `transactions` | 303 | 90 | 财务交易 |
| `daily_reviews` | `reviews` | 0 | 1 | 每日复盘 |
| `settings` | `settings` | 1 | 1 | 偏好设置（`preferences`） |
| `finance_accounts` | `accounts` | 4 | 5 | 财务账户 |
| `workout_history` | `workoutHistory` | 3 | 0 | 训练历史摘要 |

#### 笔记表 —— 创建于 `src-tauri/src/server/notes.rs::ensure_schema`（约 L192）

统一结构同 `(id, data_json, updated_at)`，启动时会种子化 6 个默认文件夹（工作/学习/健身/生活/财务/项目）。

| 表名 | 真实库行数 | 说明 |
|---|---|---|
| `notes_v2` | 6 | 笔记正文 + 元数据全部在 JSON |
| `note_folders_v2` | 12 | 文件夹 |
| `note_tags_v2` | 0 | 标签 |
| `note_revisions_v2` | 0 | 版本历史 |

#### 英语表 —— 创建于 `src-tauri/src/server/english.rs::ensure_schema`（约 L140）

`ENTITY_TABLES` 常量内 8 张表统一结构 `(id, data_json, updated_at)`，另有 `english_preferences`（key/value_json）以及种子文章（3 篇本地）和种子数据源（3 个 VOA）。

| 前端键 | 表名 | 真实库行数 | 说明 |
|---|---|---|---|
| `articles` | `english_articles` | 688 | 英语文章（含正文 content） |
| `records` | `english_learning_records` | 3 | 学习记录 |
| `vocabulary` | `english_user_vocabulary` | 1 | 用户生词 |
| `highlights` | `english_highlights` | 4 | 阅读高亮 |
| `notes` | `english_notes` | 3 | 阅读笔记 |
| `analysis` | `english_ai_analysis` | 0 | AI 分析 |
| `sources` | `english_sources` | 3 | 内容源 |
| `tasks` | `english_sync_tasks` | 24 | 同步任务 |
| – | `english_preferences` | 1 | 生词设置 key/value_json |

注意：英语表名来自 `english.rs` 的 `ENTITY_TABLES`，其中 `vocabulary` 对应 `english_user_vocabulary`。

#### 训记表 —— 创建于 `src-tauri/src/server/xunji.rs::ensure_schema`（约 L481）

| 表名 | 真实库行数 | 说明 |
|---|---|---|
| `workout_import_records` | 1 | 训记导入记录 |
| `training_notes` | 1 | 训练笔记 |

结构同为 `(id, data_json, updated_at)`。

#### 导入上传表 —— 创建于 `src-tauri/src/server/imports.rs::ensure_schema`（约 L85）

| 表名 | 真实库行数 | 说明 |
|---|---|---|
| `import_uploads` | 1 | 账单/训练导入文件元数据 |

结构同为 `(id, data_json, updated_at)`，文件本体保存在 `data_dir/imports/`。

#### AI 管家表 —— 创建于 `src-tauri/src/server/assistant.rs::ensure_schema`（约 L233）

| 表名 | 真实库行数 | 结构 |
|---|---|---|
| `ai_settings` | 1 | `(id, data_json, updated_at)`，固定 id=`deepseek` |
| `ai_conversations` | 1 | 真实列 `(id, title, messages_json, created_at, updated_at)` + updated_at 索引 |

#### 翻译表 —— 创建于 `src-tauri/src/server/translation.rs::ensure_schema`（约 L47）

| 表名 | 真实库行数 | 结构 |
|---|---|---|
| `translation_settings` | 1 | `(id, data_json, updated_at)`，固定 id=`baidu` |

#### 照片同步表 —— 创建于 `src-tauri/src/server/photo.rs::ensure_schema`（约 L195）

照片模块已经是真实列结构（本仓库内唯一非 JSON 业务模块）：

| 表名 | 真实库行数 | 主要字段 |
|---|---|---|
| `photos` | 0 | id, content_hash(UNIQUE), original_file_name, stored_file_name, original_path, thumbnail_path, media_type, mime_type, file_size, width, height, duration_ms, captured_at, imported_at, processing_status, processing_error, source_device_id, deleted_at |
| `photo_sync_devices` | 0 | id, device_name, device_type, device_uuid(UNIQUE), token_hash(UNIQUE), status, paired_at, last_seen_at, revoked_at |
| `photo_upload_tasks` | 0 | id, device_id, client_asset_id, …, status, photo_id, created_at/updated_at/expires_at, error_code/error_message, is_duplicate, UNIQUE(device_id, client_asset_id) |
| `photo_device_assets` | 0 | device_id, client_asset_id, photo_id, synced_at, UNIQUE(device_id, client_asset_id) |

另有索引 `photos_captured_at_idx`、`photo_tasks_status_idx`。

#### 迁移元数据表 —— 创建于 `src-tauri/src/server/migration.rs::migrate_once`（约 L188）

| 表名 | 说明 |
|---|---|
| `app_meta` | `(key TEXT PRIMARY KEY, value TEXT)`；当前只写 `legacy_d1_migration` 标记 |

### 2.2 当前真实数据库表结构快照

对本机 `%APPDATA%\com.lifetrace.desktop\lifetrace.db` 只读枚举结果（共 31 张业务/元数据表）与上表一致：所有核心业务表均为 `(id, data_json, updated_at)`，仅照片、AI 会话、`english_preferences`、`app_meta` 使用真实列。

---

## 3. JSON 内真实业务字段

以下字段全部来自 `src/types/*.ts` 与真实库样本 JSON，是后续真实列化的依据。

### 3.1 Activity（`activities`）

```json
{
  "id": "piano", "userId": "local-user", "name": "钢琴练习",
  "type": "duration", "unit": "分钟", "minimumTarget": 10, "normalTarget": 30,
  "targetPeriod": "daily", "icon": "music", "isArchived": false,
  "createdAt": "...", "updatedAt": "..."
}
```

可选：`targetDays`、`color`、`scheduleType`、`startDate`、`checkinMethod`、`syncSource`、`description`。

### 3.2 ActivityLog（`activity_logs`）

```json
{
  "id": "...", "userId": "local-user", "activityId": "piano",
  "value": 15, "status": "completed", "createdAt": "...", "updatedAt": "..."
}
```

可选：`note`、`metadata`（`{state, urgeLevel, triggers, actions}`）。

### 3.3 Transaction（`transactions`）

```json
{
  "id": "...", "userId": "local-user",
  "occurredAt": "2026-07-23T09:00:24.000Z", "createdAt": "...", "updatedAt": "...",
  "type": "expense", "amount": 4, "category": "转账与人情", "account": "微信零钱",
  "accountId": "wechat-wallet", "counterparty": "...", "item": "...", "note": "..."
}
```

可选：`toAccount`、`toAccountId`。金额为 JavaScript `number`（f64），是必须转为 `amount_cents` 的字段。

### 3.4 FinanceAccount（`finance_accounts`）

```json
{
  "id": "wechat-wallet", "userId": "local-user", "name": "微信零钱",
  "type": "wechat", "balance": 128.31, "last4": "", "color": "#2a9c69",
  "icon": "微", "isArchived": false, "createdAt": "...", "updatedAt": "..."
}
```

可选：`balanceAt`。`balance` 可为 `null`。

### 3.5 DailyReview（`daily_reviews`）

```json
{
  "id": "...", "userId": "local-user", "reviewDate": "2026-07-22",
  "energy": 7, "mood": 7, "bestThing": "", "problem": "",
  "tomorrowPriority": "", "note": "", "createdAt": "...", "updatedAt": "..."
}
```

可选：`completionScore`。

### 3.6 WorkoutHistory（`workout_history`）

```json
{
  "id": "xunji-...", "userId": "local-user", "templateId": "", "name": "训练标题",
  "occurredAt": "...", "durationSeconds": 3600, "exerciseCount": 4, "setCount": 20,
  "status": "completed", "source": "xunji", "sourceId": "...",
  "caloriesKcal": 300, "volumeKg": 5000,
  "exercises": [{ "name": "深蹲", "plannedSets": 5, "completedSets": 5,
    "sets": [{ "weight": 80, "reps": 8, "completed": true }] }],
  "createdAt": "...", "updatedAt": "..."
}
```

### 3.7 WorkoutImportRecord（`workout_import_records`）

```json
{
  "id": "...", "userId": "local-user", "source": "xunji", "shareUrl": "https://...",
  "rawData": { "第三方原始 JSON" }, "workout": { "date": "YYYY-MM-DD", "title": "...",
  "durationMinutes": 60, "caloriesKcal": 0, "volumeKg": 0,
  "exercises": [{ "name": "...", "sets": [{ "weightKg": 80, "reps": 8, "setNumber": 1 }] }] },
  "status": "pending|success|failed", "error": "...", "workoutRecordId": "...",
  "createdAt": "...", "updatedAt": "..."
}
```

### 3.8 TrainingNote（`training_notes`）

```json
{
  "id": "training-note-xunji-...", "userId": "local-user", "title": "2026-07-24 训练记录",
  "content": "训练：...\n日期：...", "workoutRecordId": "xunji-...",
  "source": "xunji", "noteDate": "2026-07-24", "createdAt": "...", "updatedAt": "..."
}
```

### 3.9 Note（`notes_v2`）

```json
{
  "id": "...", "title": "...", "noteType": "document", "folderId": null,
  "contentJson": { "type": "doc", "content": [] }, "contentHtml": "", "contentText": "",
  "contentMarkdown": "", "summary": "", "isPinned": false, "isFavorite": false,
  "isArchived": false, "createdAt": "...", "updatedAt": "...", "deletedAt": null,
  "version": 1,
  "tags": [{ "id": "...", "name": "...", "color": "...", "createdAt": "...", "updatedAt": "..." }],
  "relations": [{ "id": "...", "noteId": "...", "entityType": "habit", "entityId": "...",
    "relationType": "reference", "createdAt": "..." }],
  "attachments": [{ "id": "...", "noteId": "...", "fileName": "...", "originalName": "...",
    "mimeType": "...", "fileSize": 123, "storagePath": "...", "createdAt": "..." }]
}
```

可选：`aiSummary`、`aiTags`、`embeddingStatus`、`lastAiProcessedAt`。

### 3.10 NoteFolder / NoteTag / NoteRevision / NoteRelation / NoteAttachment

分别保存在 `note_folders_v2`、`note_tags_v2`、`note_revisions_v2` 的 JSON 中；关系（`NoteRelation`）和附件（`NoteAttachment`）当前**内嵌在笔记 JSON 的 `relations` / `attachments` 数组里**，没有独立表。

### 3.11 英语实体

- `EnglishArticle`（`english_articles`）：id、title、level、category、content（全文）、vocabulary[]、questions[]、difficulty、estimatedMinutes、createdTime、updatedAt、source、sourceKey、sourceName、sourceCategory、sourceUrl、normalizedSourceUrl、externalId、publishedAt、sourceUpdatedAt、imageUrl、audioUrl、author、summary、wordCount、fetchedAt、rightsNote、contentHash、language、qualityScore、hasAudio、licenseType、attribution、processingStatus、fetchStatus、retryCount、lastError，读取时附加 readingStatus/completedAt。
- `EnglishLearningRecord`：id、userId、date、articleId、readingTimeSeconds、summary、score、analysisId、newWords[]、completionStatus、readingStatus、startedAt、completedAt、createdAt、updatedAt。
- `UserVocabulary`（`english_user_vocabulary`）：id、userId、word、normalizedWord、lemma、dictionaryWordId、phonetic、selectedMeanings[]、partOfSpeech、sourceArticleId、sourceArticleTitle、sourceSentence、notes、masteryLevel、reviewStage、reviewCount、correctCount、incorrectCount、encounterCount、lastReviewedAt、nextReviewAt、status（LEARNING/REVIEWING/MASTERED/ARCHIVED）、frequencyRank、tags[]、occurrences[]（内嵌）、reviewLogs[]（内嵌）。
- `VocabularyOccurrence` / `VocabularyReviewLog`：当前**内嵌**在生词 JSON 的 `occurrences` / `reviewLogs` 数组中。
- `EnglishHighlight`：id、userId、articleId、text、color、blockId、startOffset、endOffset、selectedText、prefix、suffix、createdAt、updatedAt。
- `EnglishNote`：id、userId、articleId、quote、content、blockId、startOffset、endOffset、selectedText、prefix、suffix、highlightId、createdAt、updatedAt。
- `EnglishAIAnalysis`：id、userId、recordId、articleId、provider、score、contentScore、grammarScore、vocabularyScore、structureScore、mistakes[]、suggestions[]、improvedSummary、weakPoints[]、createdAt、updatedAt。
- `EnglishContentSourceState`（`english_sources`）：id、sourceKey、sourceName、sourceType、sourceUrl、category、enabled、syncInterval、initialFetchLimit、recentScanLimit、overlapDays、requestIntervalMs、lastSyncAt、lastSuccessAt、lastNewArticleAt、latestExternalPublishedAt、syncCursor、consecutiveFailures、status、lastError、articleCount、createdAt、updatedAt。
- `EnglishSyncTask`（`english_sync_tasks`）：以 `taskId` 作为 id；taskType、sourceKey、requestedLimit、status、startedAt、finishedAt、各计数、currentArticle、progress、lastError、createdAt、updatedAt。

### 3.12 导入上传 / AI / 翻译设置

- `ImportUpload`（`import_uploads`）：id、kind（fitness/bill）、filename、contentType、size、status（pending/parsed）、objectKey、createdAt、updatedAt。
- `ai_settings`：固定 id=`deepseek`，JSON 保存 AI 服务配置（baseUrl、apiKey、model、enabled 等）。
- `translation_settings`：固定 id=`baidu`，JSON 保存百度翻译配置（appId、secretKey 等）。
- `settings`（`preferences`）：`{ id: "preferences", dark: bool, timer: {activityId, startedAt, accumulatedSeconds} | null, updatedAt }`。

---

## 4. API 与前端读写入口

### 4.1 统一状态协议 `/api/state`

实现：`src-tauri/src/server/state.rs`；前端封装：`src/db/sqliteClient.ts`；状态层：`src/stores/useLifeStore.ts`。

| 接口 | 操作 | 行为 |
|---|---|---|
| `GET /api/state` | 读 | 遍历 7 张表 `SELECT data_json ORDER BY updated_at DESC`，返回 `{activities, logs, transactions, reviews, settings, accounts, workoutHistory}` |
| `POST /api/state` | `put` | `INSERT ... ON CONFLICT(id) DO UPDATE` 整个 DTO 序列化写入 `data_json` |
| `POST /api/state` | `patch` | 仅 `activities`/`accounts`；读取 JSON、合并 patch、整体写回 |
| `POST /api/state` | `delete` | 按 id 物理删除 |
| `POST /api/state` | `restore` | 事务内清空 6 张业务表（不含 settings）后按数组重写 |

前端写入口（`useLifeStore`）：

- `addLog` / `addTransaction` / `saveReview` / `addActivity` / `updateActivity` / `archiveActivity` / `saveAccount` / `deleteAccount` / `deleteWorkoutHistory` / `updateTransaction` / `deleteTransaction` / `startTimer` / `pauseTimer` / `finishTimer` / `restoreBackup` / `toggleDark`

### 4.2 笔记 `/api/notes`

实现：`src-tauri/src/server/notes.rs`；前端封装：`src/services/noteApi.ts`（供 `NotesModule.tsx` 使用）。

| 接口 | 操作 |
|---|---|
| `GET /api/notes` | `list`（默认，内存过滤搜索/排序/分页）、`get`、`meta`、`revisions`、`backup` |
| `POST /api/notes` | `create`、`update`、`trash`、`restore`、`delete`、`duplicate`、`folder.save`、`folder.delete`、`tag.save`、`tag.delete`、`revision.restore`、`attachment.record`、`attachment.delete`、`backup.restore` |

当前列表查询会把每张笔记的完整 JSON（含 `content_json`）读入内存再过滤。

### 4.3 英语 `/api/english/{*path}`

实现：`src-tauri/src/server/english.rs::dispatch`（另含 `/api/english/dictionary/lookup` 与 `/api/english/translate`）。

主要路径：`today`、`history`、`assistant`、`articles`、`articles/stats`、`highlights`、`notes`、`reading`、`summary`、`analyze`、`vocabulary`、`vocabulary/stats`、`vocabulary/settings`、`vocabulary/{id}`、`vocabulary/{id}/review`、`vocabulary/{id}/occurrences`、`sources`、`sources/{key}`、`sync`、`sync/status`、`sync/logs`。

前端入口：`src/components/english/DailyEnglish.tsx`、`VocabularyWorkspace.tsx`、`DictionaryPopover.tsx` 等。

### 4.4 训记 `/api/xunji/*`

实现：`src-tauri/src/server/xunji.rs`；前端：`XunjiImportPanel.tsx`。

| 接口 | 操作 |
|---|---|
| `POST /api/xunji/parse` | 上传图片 → 二维码识别 → 抓取分享页 → 解析 → 写 `workout_import_records` |
| `GET /api/xunji/imports` | 读取导入记录列表 |
| `POST /api/xunji/imports` | `confirm` 时写 `workout_history`、自动习惯打卡（`activities`/`activity_logs`）、`training_notes`；`cancel` 标记失败 |

### 4.5 导入 `/api/imports`

实现：`src-tauri/src/server/imports.rs`；上传文件存 `data_dir/imports/{kind}/{id}/...`。

### 4.6 AI / 翻译 / 照片

- `/api/assistant/catalog`、`/api/assistant/chat`、`/api/assistant/conversations`、`/api/settings/ai`（`assistant.rs`）
- `/api/settings/translation`、`/api/english/translate`（`translation.rs`）
- `/api/photo-sync/dashboard`（`photo.rs`）+ 局域网 3443/3444/3445 端口服务

### 4.7 备份与恢复入口

- 前端「数据设置」页（`src/components/HengXuShell.tsx`）：
  - 导出：`{ exportedAt, activities, logs, transactions, reviews, accounts, workoutHistory, notesBackup: { format: "lifetrace-notes", version: 2, ... } }`
  - 恢复：`store.restoreBackup(data)`（POST /api/state restore）→ 若含 `notesBackup` 再调 `noteApi.restoreBackup`（POST /api/notes backup.restore）
- 旧桌面备份文件：`backups/lifetrace-before-clear-20260724-0221.sqlite`（直接 SQLite 文件拷贝）
- 旧 D1 状态备份：`backups/wrangler-state-before-action-library-cleanup-20260726/v3/d1/miniflare-D1DatabaseObject/*.sqlite`

---

## 5. 当前 Migration 路径

### 5.1 启动顺序（`src-tauri/src/server.rs::serve`）

```text
打开 lifetrace.db（Connection::open）
→ PRAGMA journal_mode=WAL / foreign_keys=ON / busy_timeout=5000
→ state::ensure_schema   （核心 7 表）
→ assistant::ensure_schema
→ imports::ensure_schema
→ notes::ensure_schema   （笔记 4 表 + 默认文件夹种子）
→ xunji::ensure_schema
→ translation::ensure_schema
→ english::ensure_schema （英语 8 表 + preferences + 文章/数据源种子）
→ photo::ensure_schema   （真实列）
→ migration::migrate_once（旧 D1 数据导入，失败仅打印日志不阻断启动）
→ 启动 HTTP 服务（127.0.0.1:3103）
```

### 5.2 旧数据导入（`src-tauri/src/server/migration.rs`）

- 首次运行建 `app_meta` 表；`legacy_d1_migration` 标记存在则跳过。
- 扫描目录：`%APPDATA%\LifeTrace\wrangler-state`、`%APPDATA%\lifetrace\wrangler-state`、`{项目目录}\.wrangler\state`（递归深度 ≤7，跳过 `metadata.sqlite`，只取 >64KB 的 `.sqlite`）。
- 按修改时间倒序，找到含目标表的第一个库开始导入；`INSERT OR IGNORE` 复制 `(id, data_json, updated_at)`。
- 12 张 JSON 表：`activities`、`activity_logs`、`transactions`、`daily_reviews`、`settings`、`finance_accounts`、`workout_history`、`english_articles`、`english_learning_records`、`english_highlights`、`english_notes`、`english_ai_analysis`。
- 笔记：D1 的 `notes` 为真实列，用 SQL `json_object(...)` 组装 JSON 写入 `notes_v2`；`note_folders`→`note_folders_v2`、`note_tags`→`note_tags_v2`、`note_revisions`→`note_revisions_v2`（仅复制表存在的部分）。
- 旧库**不会被删除或改写**；迁移失败只 `eprintln!` 后继续启动。

### 5.3 已确认的迁移缺口（风险）

- D1 中以下真实列/JSON 表**不在** `JSON_TABLES` 列表，不会被导入：`english_user_vocabulary`（生词）、`english_vocabulary_occurrences`、`english_vocabulary_review_logs`、`english_content_sources`、`english_sync_tasks`、`english_sync_logs`、`english_processing_queue`、`english_library_state`、`import_uploads`、`workout_import_records`、`training_notes`、`workout_templates`、`exercise_library`、`note_attachments`、`note_relations`、`note_tag_relations`。
- 笔记导入只复制 4 张 v2 表，`note_relations` / `note_attachments` / `note_tag_relations` 这些 D1 真实列关系表被丢弃，仅通过 `notes` 的 `json_object` 内嵌部分 tags/relations/attachments（`note_attachments` 未出现在组装 SQL 中，实际会丢失附件元数据）。
- `english_articles` 在 D1 中同时有真实列与 `data_json`，当前只复制 `data_json`（若该列未同步更新则数据不一致）。

---

## 6. 旧数据库与备份格式

### 6.1 旧桌面 JSON 库（`backups/lifetrace-before-clear-20260724-0221.sqlite`）

- 1.2MB，WAL 之前直接拷贝的快照。
- 表：`activities`(5)、`activity_logs`(13)、`daily_reviews`(1)、`finance_accounts`(5)、`settings`(1)、`transactions`(90)、`workout_history`(0)、`exercise_library`(873)、`workout_templates`(2)、`app_meta`(1)、`_cf_METADATA`(1)。
- 全部业务表为 `(id, data_json, updated_at)` JSON 结构。

### 6.2 旧 D1 库（`backups/wrangler-state-.../v3/d1/miniflare-D1DatabaseObject/faaf2b04....sqlite`）

- 8.7MB，包含真实列与 JSON 混合结构：
  - JSON 表：`activities`(2)、`activity_logs`(3)、`daily_reviews`(0)、`finance_accounts`(4)、`settings`(1)、`transactions`(100)、`workout_history`(2)、`workout_import_records`(13)、`training_notes`(2)、`english_ai_analysis`(0)、`english_highlights`(0)、`english_learning_records`(0)、`english_notes`(0)、`english_vocabulary`(0)、`english_vocabulary_settings`(0)、`import_uploads`(0)
  - 真实列表：`notes`(5)、`note_folders`(6)、`note_tags`(0)、`note_tag_relations`(0)、`note_relations`(3)、`note_revisions`(0)、`note_attachments`(0)、`english_articles`(685，双结构)、`english_content_sources`(5)、`english_library_state`(1)、`english_processing_queue`(457)、`english_sync_logs`(449)、`english_sync_tasks`(10)、`english_user_vocabulary`(1)、`english_vocabulary_occurrences`(1)、`english_vocabulary_review_logs`(0)、`notes_fts*`（FTS5）

### 6.3 当前 JSON 备份格式（前端导出）

```json
{
  "exportedAt": "ISO8601",
  "activities": [...], "logs": [...], "transactions": [...],
  "reviews": [...], "accounts": [...], "workoutHistory": [...],
  "notesBackup": { "format": "lifetrace-notes", "version": 2, "createdAt": "...",
    "notes": [...], "folders": [...], "tags": [...], "revisions": [...] }
}
```

注意：当前导出**没有**顶层 `format` / `schemaVersion` 标识；恢复不区分版本，也不在恢复前备份数据库。

---

## 7. 风险点清单

| # | 风险 | 影响 | 当前应对 |
|---|---|---|---|
| R1 | 核心业务表全部为 `(id, data_json, updated_at)`，无约束/索引/外键 | 高频查询（按日期、账户、状态）全表扫描 JSON；无法保证关系完整性 | 无 |
| R2 | 金额用 f64（`amount: number`） | 浮点误差；对账不可靠 | 前端 `finance.ts` 自行计算余额 |
| R3 | 无版本化 Migration，`ensure_schema` 分散在 8 个模块 | 无法审计 schema 变更；无法回滚 | `app_meta` 只有一个标记 |
| R4 | `migration.rs` 在 `ensure_schema` 之后运行且失败仅打日志 | 数据未导入但应用照常启动，用户不知情 | 无 |
| R5 | D1 导入 `INSERT OR IGNORE` 静默跳过冲突 id | 潜在静默丢数据 | 无 |
| R6 | D1 导入表清单不全（生词、数据源、同步任务、导入记录、训练笔记、附件/关系表缺失） | 老用户生词/关系/附件数据可能丢失 | 无 |
| R7 | `notes` 的 `json_object` 组装不包含 `note_attachments` | 附件元数据迁移缺失 | 无 |
| R8 | 笔记列表/`meta`/`backup` 全部读取完整 JSON（含 content_json）到内存再过滤 | 笔记多时内存与延迟恶化；FTS 未使用（D1 曾有 `notes_fts`） | 无 |
| R9 | 英语列表（`articles`、`vocabulary`、`history`、`stats`）同样全量读 JSON | 688 篇文章全量解析，量大后不可扩展 | 无 |
| R10 | 同日多条复盘无唯一约束；前端按 `reviewDate` 覆盖 | 数据重复或静默覆盖 | 前端 `saveReview` 按日期过滤 |
| R11 | `restore`/`backup.restore` 先清空再写入，恢复前无数据库备份 | 恢复中断即丢数据 | 在事务中执行（部分缓解） |
| R12 | WAL 模式下备份必须用 Backup API / `VACUUM INTO`，当前无备份实现 | 直接拷贝主库会得到不一致快照 | 无 |
| R13 | 旧 ID 约定特殊（`xunji-{sourceId}`、`reading-{articleId}`、`workout-log-...`、`training-note-...`、`voa-{hash}`） | 迁移必须原样保留 id，否则自动打卡/去重/笔记关联断裂 | 无 |
| R14 | `english_sync_tasks` 用 `taskId` 作为主键值；`assistant.rs` 日期表达式里写的是 `english_analysis`（与真实表 `english_ai_analysis` 不符） | 潜在错误引用 | 无 |
| R15 | 财务 `category`/`account` 目前是冗余字符串字段，与 `accountId` 并存 | 规范化后需保留 `legacy_category_name`/`legacy_account_name` 以兼容旧 UI | 无 |
| R16 | 旧备份无 `format`/`schemaVersion` 标识 | 导入新版本需要版本识别与统一转换 | 无 |
| R17 | 前端 `patch` 仅支持 activities/accounts；settings 用整对象 `put` | 兼容层必须保持这些行为 | 无 |
| R18 | 无 `PRAGMA integrity_check` / `foreign_key_check` 校验 | 无法在迁移前后证明数据一致性 | 无 |
| R19 | `restore` 的 `DELETE FROM {table}` 依赖白名单 `TABLES` 常量 | 表名单需与 Repository 同步维护，否则白名单漂移 | 白名单校验（部分缓解） |
| R20 | 真实库 `workout_history` 0 行但 `workout_import_records`/`training_notes` 有数据；`notes_v2` 6 条但 `note_tags_v2` 0 条 | 迁移必须容忍空表与缺失关系 | 无 |

---

## 8. 对 EPIC-01 的结论

1. 需要迁移的**核心 JSON 表**共 7 + 4（笔记）+ 8（英语）+ 2（训记）+ 1（导入）+ 2（AI/翻译设置）≈ 24 张，其中 `ai_conversations` 与照片表已是真实列，不属于本 Epic 迁移范围。
2. `settings`（preferences）与 AI/翻译凭据属于低频 key-value，适合保留 `(key, value_json)` 模式，不强制拆列。
3. 真实库当前数据量不大（303 交易、688 文章、6 笔记等），但 `backups/` 内的旧 D1 库含 685 文章、100 交易、457 处理队列等，是迁移测试的现成 fixture 来源（测试中建议程序化重建，避免提交二进制大文件）。
4. 迁移框架必须先于任何业务表改动落地，并把 `server.rs` 启动顺序改为“版本化 Migration → 非核心模块初始化 → 启动服务”，同时保留 `/api/state` 兼容层。
