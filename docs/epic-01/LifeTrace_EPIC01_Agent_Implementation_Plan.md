
# LifeTrace EPIC-01 数据层重构：Agent 具体实施方案

> 目标仓库：`zhouxingxing1279/LifeTrace`  
> 当前技术栈：Tauri 2 + React/Vite + Rust/Axum + rusqlite + SQLite  
> 任务目标：完成 EPIC-01 数据层重构，不提前实现云同步、账号系统、复杂 AI 或邮件系统。  
> 核心原则：**不丢数据、不破坏现有 UI、迁移可验证、失败可恢复、为后续同步预留字段。**

---

# 1. 最终目标

将当前主要依赖以下结构的业务表：

```sql
id TEXT PRIMARY KEY,
data_json TEXT NOT NULL,
updated_at TEXT NOT NULL
```

重构为：

- 高频查询字段使用真实列
- 金额使用整数分
- 日期、状态和关系可由数据库约束
- 关键查询字段建立索引
- 复杂原始内容继续保留 JSON
- 旧数据可自动迁移
- 迁移前后数据可校验
- 现有桌面功能不因存储重构而失效
- 为后续 EPIC-02 同步协议保留必要字段

EPIC-01 完成后，以下领域应完成规范化：

1. 财务账户、分类、交易与交易证据
2. 习惯、打卡与每日复盘
3. 笔记元数据、文件夹、标签、关系和版本
4. 英语文章、学习记录、高亮、生词和复习状态
5. 训记导入记录与训练摘要
6. Migration、备份、校验和恢复框架

---

# 2. 当前仓库基线

Agent 开始修改前，必须实际阅读：

```text
src-tauri/src/server.rs
src-tauri/src/server/state.rs
src-tauri/src/server/migration.rs
src-tauri/src/server/notes.rs
src-tauri/src/server/english.rs
src-tauri/src/server/imports.rs
src-tauri/src/server/xunji.rs
src-tauri/src/server/photo.rs
src/types/index.ts
src/types/english.ts
src/stores/useLifeStore.ts
src/services/
```

当前已知实现：

- SQLite 文件位于 `data_dir/lifetrace.db`
- 启动时启用 WAL、外键和 busy timeout
- 多个模块分别调用 `ensure_schema`
- `/api/state` 统一处理：
  - `activities`
  - `activity_logs`
  - `transactions`
  - `daily_reviews`
  - `settings`
  - `finance_accounts`
  - `workout_history`
- 这些表当前主要保存 `data_json`
- 笔记表 `notes_v2`、`note_folders_v2`、`note_tags_v2`、`note_revisions_v2` 也主要保存 `data_json`
- 当前旧版迁移主要复制 JSON 记录
- 前端 DTO 使用 camelCase
- 财务金额当前以 JavaScript `number` 表示
- Rust 数据访问使用 `rusqlite`

必须以执行时仓库代码为最终依据，并在实施报告中列出实际发现。

---

# 3. 本 Epic 的边界

## 必须完成

- 版本化 Migration Runner
- Migration 前自动备份
- 规范化真实列表结构
- 旧 JSON 数据迁移
- 数据校验报告
- 旧 API 兼容适配
- 旧 JSON 备份导入兼容
- 单元测试和集成测试
- 数据库结构文档
- Migration 失败恢复机制

## 不在本 Epic 实现

- PostgreSQL
- 云端 Push/Pull 同步
- 用户账号和设备管理
- 完整 Sync Outbox
- 冲突解决
- Android App
- 邮件系统
- AI Tool Registry
- 完整任务系统
- 对象存储

可以预留公共字段，但不能把 EPIC-01 扩张成所有后续 Epic。

---

# 4. 硬性规则

1. 不得删除或覆盖用户原数据库文件。
2. 破坏性 Migration 前必须创建独立备份。
3. 迁移失败不得写入成功版本。
4. 无法解析的旧记录不得静默丢弃。
5. 校验通过前不得删除旧表。
6. 不得让现有 UI 因 API 字段变化直接失效。
7. 金额不得使用 SQLite `REAL`。
8. 时间点统一保存为 UTC RFC3339；业务自然日单独保存 `YYYY-MM-DD`。
9. Tiptap、第三方原始响应、OCR 原始结果允许继续使用 JSON。
10. SQL 必须参数绑定。
11. 外键测试必须在 `PRAGMA foreign_keys=ON` 下执行。
12. 不得通过清空数据库让测试通过。
13. 不得只建表而不迁移真实旧数据。
14. 不得把旧 JSON 原样塞入新表后声称完成规范化。
15. 每个 Migration 只能执行一次，并有 checksum。
16. 旧 ID 原样保留，避免关系断裂。
17. 所有迁移错误必须包含表名、记录 ID 和原因。
18. 完成后必须执行前端测试、Rust 测试和至少一次旧数据库迁移演练。

---

# 5. 总体实施策略

```text
读取当前 schema
→ 创建迁移前备份
→ 建立 schema_migrations
→ 创建规范化新表
→ 分领域迁移旧 JSON
→ 校验记录数、金额、关系和约束
→ 切换 Repository
→ 保留 API 兼容层
→ 标记 Migration 成功
→ 保留旧表一个兼容版本
```

禁止直接：

```sql
DROP TABLE transactions;
```

推荐：

```text
旧 transactions
→ legacy_transactions_json_v1

创建新 transactions
→ 迁移
→ 校验
→ Repository 切换
```

不要长期双写新旧表。旧表只用于迁移回溯和兼容观察。

---

# 6. 推荐代码结构

```text
src-tauri/src/database/
├── mod.rs
├── connection.rs
├── backup.rs
├── migration_runner.rs
├── schema.rs
├── validation.rs
│
├── migrations/
│   ├── mod.rs
│   ├── m0001_framework.rs
│   ├── m0002_finance.rs
│   ├── m0003_habits_reviews.rs
│   ├── m0004_notes.rs
│   ├── m0005_english.rs
│   └── m0006_workouts_imports.rs
│
├── legacy/
│   ├── mod.rs
│   ├── json_parser.rs
│   ├── d1_import.rs
│   ├── backup_v1.rs
│   └── report.rs
│
└── repositories/
    ├── mod.rs
    ├── state_compat.rs
    ├── finance.rs
    ├── habits.rs
    ├── reviews.rs
    ├── notes.rs
    ├── english.rs
    └── workouts.rs
```

允许根据现有结构调整，但必须保证：

- Connection 配置只有一处
- Migration 只有一个入口
- 数据库备份只有一套实现
- 旧 JSON 解析逻辑可复用
- 业务模块不再自行创建核心业务表
- Handler 不直接承担 Migration
- Repository 负责数据库与 DTO 转换

---

# 7. Migration 框架

## 7.1 元数据表

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    checksum TEXT NOT NULL,
    applied_at TEXT NOT NULL,
    app_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS migration_runs (
    id TEXT PRIMARY KEY,
    from_version INTEGER NOT NULL,
    to_version INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('running', 'succeeded', 'failed')
    ),
    backup_path TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    error_message TEXT
);

CREATE TABLE IF NOT EXISTS migration_issues (
    id TEXT PRIMARY KEY,
    migration_run_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    severity TEXT NOT NULL CHECK (
        severity IN ('warning', 'error')
    ),
    message TEXT NOT NULL,
    raw_json TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (migration_run_id)
        REFERENCES migration_runs(id)
        ON DELETE CASCADE
);
```

## 7.2 Migration 接口

```rust
pub trait Migration {
    fn version(&self) -> i64;
    fn name(&self) -> &'static str;
    fn checksum(&self) -> &'static str;

    fn up(
        &self,
        connection: &mut rusqlite::Connection,
        context: &MigrationContext,
    ) -> Result<MigrationReport, MigrationError>;
}
```

## 7.3 Runner 流程

```text
读取当前版本
→ 查找待执行 Migration
→ 创建数据库备份
→ 写 migration_runs=running
→ BEGIN IMMEDIATE
→ 执行 Migration
→ 执行校验
→ 写 schema_migrations
→ COMMIT
→ 写 migration_runs=succeeded
```

失败：

```text
ROLLBACK
→ migration_runs=failed
→ 保留备份
→ 停止启动业务服务
→ 输出明确错误
```

## 7.4 server.rs 启动顺序

修改为：

```text
打开数据库
→ 设置 PRAGMA
→ database::run_migrations()
→ legacy D1 导入（仅未导入时）
→ 初始化非核心缓存表
→ 启动 HTTP 服务
```

旧 `ensure_schema` 不再创建核心业务表。

---

# 8. Migration 前备份

## 8.1 路径

```text
%APPDATA%\com.lifetrace.desktop\backups\database\
```

文件：

```text
lifetrace-before-schema-v{version}-{timestamp}.db
```

## 8.2 备份方式

禁止 WAL 状态下只复制主 `.db` 文件。

优先：

```rust
rusqlite::backup::Backup
```

也可使用：

```sql
VACUUM INTO ?
```

完成后：

- 打开备份数据库
- 执行 `PRAGMA integrity_check`
- 必须返回 `ok`
- 记录文件大小
- 计算 SHA-256
- 保存到 migration run

## 8.3 保留策略

- 至少保留最近 3 次 Migration 前备份
- 最近一次成功 Migration 备份不得自动删除
- 清理失败不阻塞应用启动
- 日志中输出备份路径

---

# 9. 公共实体字段

主实体统一包含：

```sql
id TEXT PRIMARY KEY,
user_id TEXT NOT NULL DEFAULT 'local',
created_at TEXT NOT NULL,
updated_at TEXT NOT NULL,
deleted_at TEXT,
version INTEGER NOT NULL DEFAULT 1,
modified_by_device TEXT
```

EPIC-01 中：

- `user_id='local'`
- `modified_by_device` 可为空
- 不实现设备注册
- 每次正常修改 `version + 1`
- 删除优先软删除

时间：

- 时间点：UTC RFC3339
- 本地自然日：`YYYY-MM-DD`
- 原始时间无法确定时记录 migration issue

---

# 10. 财务规范化

## 10.1 finance_accounts

```sql
CREATE TABLE finance_accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL DEFAULT 'local',
    name TEXT NOT NULL,
    account_type TEXT NOT NULL CHECK (
        account_type IN (
            'cash', 'bank', 'wechat',
            'alipay', 'investment', 'other'
        )
    ),
    opening_balance_cents INTEGER,
    balance_at TEXT,
    last4 TEXT,
    color TEXT NOT NULL,
    icon TEXT NOT NULL,
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (
        is_archived IN (0, 1)
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    modified_by_device TEXT
);
```

旧 `balance`：

```text
null → NULL
number → ROUND(number * 100)
NaN/Infinity → error
```

## 10.2 transaction_categories

```sql
CREATE TABLE transaction_categories (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL DEFAULT 'local',
    name TEXT NOT NULL,
    category_type TEXT NOT NULL CHECK (
        category_type IN ('expense', 'income', 'transfer')
    ),
    parent_id TEXT,
    icon TEXT,
    color TEXT,
    is_system INTEGER NOT NULL DEFAULT 0,
    is_archived INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    modified_by_device TEXT,
    FOREIGN KEY (parent_id)
        REFERENCES transaction_categories(id)
);

CREATE UNIQUE INDEX uq_transaction_categories_name
ON transaction_categories(
    user_id,
    category_type,
    name
)
WHERE deleted_at IS NULL;
```

迁移：

- 收集旧分类字符串
- 相同交易类型和规范化名称只建一次
- 无分类归入“未分类”
- 保存 `legacy_category_name`

## 10.3 transactions

```sql
CREATE TABLE transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL DEFAULT 'local',

    transaction_type TEXT NOT NULL CHECK (
        transaction_type IN (
            'expense', 'income', 'transfer',
            'refund', 'fee'
        )
    ),

    amount_cents INTEGER NOT NULL CHECK (
        amount_cents >= 0
    ),

    currency TEXT NOT NULL DEFAULT 'CNY',

    account_id TEXT,
    to_account_id TEXT,
    category_id TEXT,

    counterparty TEXT,
    merchant TEXT,
    item TEXT,
    note TEXT,

    occurred_at TEXT NOT NULL,
    local_date TEXT NOT NULL,

    status TEXT NOT NULL DEFAULT 'confirmed' CHECK (
        status IN (
            'candidate',
            'provisional',
            'confirmed',
            'ignored'
        )
    ),

    source_type TEXT NOT NULL DEFAULT 'manual',
    external_transaction_id TEXT,
    legacy_category_name TEXT,
    legacy_account_name TEXT,
    raw_json TEXT,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    modified_by_device TEXT,

    FOREIGN KEY (account_id)
        REFERENCES finance_accounts(id),
    FOREIGN KEY (to_account_id)
        REFERENCES finance_accounts(id),
    FOREIGN KEY (category_id)
        REFERENCES transaction_categories(id)
);

CREATE INDEX idx_transactions_date
ON transactions(user_id, local_date, deleted_at);

CREATE INDEX idx_transactions_account
ON transactions(user_id, account_id, occurred_at);

CREATE INDEX idx_transactions_category
ON transactions(user_id, category_id, occurred_at);

CREATE UNIQUE INDEX uq_transactions_external
ON transactions(
    user_id,
    source_type,
    external_transaction_id
)
WHERE external_transaction_id IS NOT NULL
  AND deleted_at IS NULL;
```

转账校验：

- `account_id` 非空
- `to_account_id` 非空
- 两者不能相同

## 10.4 transaction_evidence

```sql
CREATE TABLE transaction_evidence (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_id TEXT,
    external_transaction_id TEXT,
    confidence REAL,
    raw_json TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (transaction_id)
        REFERENCES transactions(id)
        ON DELETE CASCADE
);
```

旧交易生成：

```text
source_type=legacy_json
```

## 10.5 财务校验

必须对比：

- 旧交易总数
- 新交易总数
- 支出总金额
- 收入总金额
- 转账总金额
- 每月收入和支出
- 每账户交易数量
- 无法匹配账户数量
- 无法匹配分类数量

金额差值必须为：

```text
0 cents
```

---

# 11. 习惯与复盘

## 11.1 activities

真实列：

```text
id
user_id
name
activity_type
unit
minimum_target
normal_target
target_period
schedule_type
start_date
checkin_method
sync_source
description
icon
color
is_archived
created_at
updated_at
deleted_at
version
modified_by_device
```

允许 JSON：

```text
target_days_json
复杂 schedule
低频 metadata
```

## 11.2 activity_logs

```sql
CREATE TABLE activity_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL DEFAULT 'local',
    activity_id TEXT NOT NULL,
    log_date TEXT NOT NULL,
    value REAL,
    status TEXT,
    note TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    modified_by_device TEXT,
    FOREIGN KEY (activity_id)
        REFERENCES activities(id)
);

CREATE INDEX idx_activity_logs_activity_date
ON activity_logs(activity_id, log_date, deleted_at);
```

## 11.3 daily_reviews

```sql
CREATE TABLE daily_reviews (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL DEFAULT 'local',
    review_date TEXT NOT NULL,
    energy INTEGER CHECK (energy BETWEEN 1 AND 10),
    mood INTEGER CHECK (mood BETWEEN 1 AND 10),
    completion_score REAL,
    best_thing TEXT,
    problem TEXT,
    tomorrow_priority TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    modified_by_device TEXT
);

CREATE UNIQUE INDEX uq_daily_reviews_date
ON daily_reviews(user_id, review_date)
WHERE deleted_at IS NULL;
```

同日重复复盘：

- 最新记录作为 active
- 其他记录不得丢弃
- 写入 migration issue
- 保留原始 JSON

---

# 12. 笔记规范化

正文继续保留 JSON/HTML/Markdown/Text，但元数据拆列。

主要表：

```text
note_folders
notes
note_tags
note_tag_relations
note_relations
note_revisions
```

`notes` 至少包含：

```text
id
user_id
title
note_type
folder_id
content_json
content_html
content_text
content_markdown
summary
is_pinned
is_favorite
is_archived
created_at
updated_at
deleted_at
version
modified_by_device
ai_summary
ai_tags_json
embedding_status
last_ai_processed_at
```

要求：

- `folder_id` 外键
- 标签关系使用关系表
- 业务实体关系使用 `note_relations`
- revision 使用 `(note_id, revision_version)` 唯一约束
- 列表查询不读取完整 `content_json`
- 详情查询才读取完整正文
- FTS5 可用时建立全文索引
- FTS5 不可用时继续使用参数化 LIKE
- FTS 失败不得让主 Migration 失败

---

# 13. 英语模块规范化

Agent 必须以当前 `english.rs` 和 `src/types/english.ts` 为准调整字段。

至少规范：

```text
english_articles
english_learning_records
english_highlights
english_notes
english_vocabulary
vocabulary_occurrences
vocabulary_review_state
english_ai_analysis
```

## 13.1 english_articles

真实列至少包括：

```text
id
title
source
source_url
level
category
content
word_count
published_at
created_at
updated_at
deleted_at
version
raw_json
```

## 13.2 english_learning_records

至少：

```text
id
user_id
article_id
status
started_at
completed_at
summary
summary_language
created_at
updated_at
deleted_at
version
```

必须建立 article 外键。

## 13.3 english_highlights

至少：

```text
id
user_id
article_id
selected_text
start_offset
end_offset
color
note
created_at
updated_at
deleted_at
version
```

## 13.4 english_vocabulary

至少：

```text
id
user_id
normalized_word
display_word
definition
phonetic
status
created_at
updated_at
deleted_at
version
metadata_json
```

唯一索引：

```sql
CREATE UNIQUE INDEX uq_english_vocabulary_word
ON english_vocabulary(user_id, normalized_word)
WHERE deleted_at IS NULL;
```

## 13.5 vocabulary_review_state

只预留数据结构，不实现完整 FSRS：

```sql
CREATE TABLE vocabulary_review_state (
    vocabulary_id TEXT PRIMARY KEY,
    due_at TEXT,
    difficulty REAL,
    stability REAL,
    retrievability REAL,
    review_count INTEGER NOT NULL DEFAULT 0,
    lapse_count INTEGER NOT NULL DEFAULT 0,
    scheduler_version TEXT,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (vocabulary_id)
        REFERENCES english_vocabulary(id)
        ON DELETE CASCADE
);
```

---

# 14. 训记与训练摘要

LifeTrace 不建设完整健身模块，只规范现有摘要和导入数据。

建议表：

```text
workout_imports
workouts
workout_exercises
workout_sets
training_notes
```

## workout_imports

至少：

```text
id
user_id
source
share_url
status
parser
parser_version
error
raw_json
workout_id
created_at
updated_at
deleted_at
version
```

## workouts

至少：

```text
id
user_id
source
source_id
name
occurred_at
local_date
duration_seconds
exercise_count
set_count
volume_kg
calories_kcal
status
raw_json
created_at
updated_at
deleted_at
version
```

## workout_exercises / workout_sets

将动作和组拆开，便于统计；无法确认的原始字段保留在 `raw_json`。

去重约束可基于：

```text
source + source_id
```

如果历史没有 source ID，不强行生成错误唯一键。

---

# 15. settings 处理

`settings` 不必完全拆成大量表。

第一版建议：

```sql
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

原因：

- 设置字段变化频繁
- 多数设置不参与统计
- 适合 Key-Value JSON

但以下未来关键配置可单独建表：

```text
AI 凭据
邮箱授权码
安全凭据
```

这些不属于普通 settings，也不在 EPIC-01 实现。

---

# 16. API 兼容层

EPIC-01 不要求立刻重写所有前端。

保留：

```text
GET /api/state
POST /api/state
```

但内部改为 Repository。

## 16.1 读取

```text
数据库真实列
→ Repository Model
→ 旧版 camelCase DTO
→ 前端
```

财务：

```text
amount_cents=3250
→ amount=32.5
```

## 16.2 写入

```text
旧前端 DTO
→ Validation
→ amount 转 cents
→ Repository
→ 新表
```

## 16.3 禁止

- 不得继续把整个 DTO 写回 `data_json`
- 不得让兼容层成为新的长期数据库模型
- 不得在 Handler 中复制大量转换逻辑

转换逻辑集中在：

```text
repositories/state_compat.rs
```

---

# 17. 备份与恢复兼容

现有 JSON 备份必须继续可用。

## 17.1 导出

短期可以继续输出旧版结构：

```text
activities
logs
transactions
reviews
settings
accounts
workoutHistory
```

内部从新表转换成旧 DTO。

同时增加：

```json
{
  "format": "lifetrace-backup",
  "schemaVersion": 2,
  "createdAt": "..."
}
```

## 17.2 导入

支持：

- 无版本旧备份
- schemaVersion=1
- schemaVersion=2

流程：

```text
解析备份
→ 识别版本
→ 转换为规范化 Import Model
→ 校验
→ 单事务写入新表
→ 输出恢复报告
```

恢复前也必须创建备份。

---

# 18. Migration 校验框架

实现：

```rust
pub struct MigrationReport {
    pub migrated: usize,
    pub skipped: usize,
    pub warnings: usize,
    pub errors: usize,
    pub metrics: BTreeMap<String, i64>,
}
```

至少执行：

```sql
PRAGMA integrity_check;
PRAGMA foreign_key_check;
```

还要执行领域校验：

## 财务

- 数量一致
- 金额一致
- account/category 外键无异常

## 习惯

- activity log 引用存在
- 日期可解析
- 同日统计一致

## 笔记

- 笔记数量一致
- 标签关系数量一致
- revision 数量一致
- content_json 是合法 JSON

## 英语

- article 引用存在
- vocabulary 去重结果可解释
- highlight offset 合法或记录 warning

## 训练

- workout、exercise、set 数量一致
- import 与 workout 关联可追溯

---

# 19. 迁移异常策略

分类：

```text
fatal
→ 整体回滚

record_error
→ 默认整体回滚，除非有明确兼容策略

warning
→ 可以继续，但必须写 migration_issues
```

禁止默认“跳过错误记录继续成功”。

允许继续的情况：

- 缺少非关键 note summary
- 无法创建 FTS
- 旧记录缺少可选颜色
- 可安全使用默认值且报告中说明

必须失败：

- 金额无法解析
- 主键为空
- 外键关系无法可靠恢复
- 数据数量异常减少
- 金额汇总不一致
- 备份失败
- integrity check 失败

---

# 20. 实施步骤

## 阶段 0：只读审计

输出：

```text
docs/epic-01/current-schema-audit.md
```

包含：

- 所有 SQLite 表
- 建表来源
- 每张表字段
- 每张表行数
- JSON 内字段样例
- API 读写入口
- 前端依赖字段
- 旧 Migration 路径
- 风险列表

本阶段不修改业务行为。

## 阶段 1：Migration 和备份框架

完成：

- `database` 模块
- `schema_migrations`
- `migration_runs`
- `migration_issues`
- Backup API
- integrity check
- Runner
- Runner 测试

不迁移业务表。

## 阶段 2：财务

完成：

- accounts
- categories
- transactions
- evidence
- 旧 JSON 迁移
- `/api/state` 财务兼容
- 财务校验和测试

财务通过后再继续。

## 阶段 3：习惯和复盘

完成：

- activities
- activity_logs
- daily_reviews
- 兼容 DTO
- 约束、索引和测试

## 阶段 4：笔记

完成：

- folders
- notes
- tags
- relations
- revisions
- 列表/详情查询优化
- FTS fallback
- 旧备份兼容

## 阶段 5：英语

完成真实列化和外键关系，不改变现有用户交互。

## 阶段 6：训记与训练摘要

仅迁移当前已有数据，不新增健身功能。

## 阶段 7：清理

- 核心模块停止调用旧 `ensure_schema`
- 删除不再使用的 JSON 写路径
- legacy 表只读
- 输出 Migration 报告
- 更新 README 和数据库文档

---

# 21. 测试要求

## 21.1 单元测试

至少覆盖：

- 金额元转分
- 分转元 DTO
- 时间解析
- 日期推导
- 分类规范化
- 账户匹配
- JSON 字段缺失
- 非法金额
- 非法状态
- 重复分类
- 软删除
- version 递增

## 21.2 Migration Fixture

建立：

```text
src-tauri/tests/fixtures/
├── legacy_empty.db
├── legacy_minimal.db
├── legacy_full.db
├── legacy_duplicate_reviews.db
├── legacy_invalid_amount.db
└── legacy_orphan_relations.db
```

如果不提交二进制数据库，可在测试代码中程序化创建 fixture。

## 21.3 集成测试

必须覆盖：

1. 空数据库首次启动
2. 当前 JSON 数据库升级
3. 旧 D1 数据库导入后再升级
4. Migration 中途失败回滚
5. 备份可打开
6. 重启不重复迁移
7. `/api/state` 返回旧 UI 兼容结构
8. 创建交易写入新表
9. 删除使用软删除
10. JSON 备份导入新表

## 21.4 回归测试

执行：

```powershell
npm.cmd test
npm.cmd run test:rust
npm.cmd run build
```

并手工检查：

- 习惯页面
- 财务页面
- 复盘页面
- 笔记页面
- 英语页面
- 训记导入
- JSON 备份和恢复

---

# 22. 性能验收

使用有代表性数据量：

```text
transactions: 50,000
activity_logs: 20,000
notes: 5,000
english_articles: 10,000
vocabulary: 20,000
```

检查：

- 月度财务统计使用索引
- 最近交易查询不扫描 JSON
- 笔记列表不读取完整正文
- activity 日期查询使用索引
- `EXPLAIN QUERY PLAN` 不出现不必要全表扫描
- Migration 不一次把所有大表加载到内存
- 使用批量事务和 prepared statement

---

# 23. 安全与可靠性

- 使用参数绑定
- 表名只能来自内部白名单
- 不把完整业务 JSON 写入普通日志
- Migration issue 中敏感内容按需脱敏
- 数据库错误不向前端暴露绝对路径
- 恢复操作必须先备份
- 写操作必须在事务中
- 连接保持 `foreign_keys=ON`
- 保持 WAL 和 busy timeout
- 禁止并发执行两个 Migration Runner

可使用文件锁或数据库锁防止多实例同时迁移。

---

# 24. 交付物

Agent 完成后必须交付：

```text
代码：
src-tauri/src/database/**
更新后的 server.rs
更新后的各 Repository / Handler
必要的 TypeScript 兼容修改

文档：
docs/epic-01/current-schema-audit.md
docs/epic-01/target-schema.md
docs/epic-01/migration-guide.md
docs/epic-01/validation-report.md
docs/epic-01/rollback-guide.md

测试：
Migration 单元测试
Fixture 迁移测试
Repository 测试
API 兼容测试
```

并提供：

- 修改文件列表
- Migration 版本列表
- 旧表和新表映射
- 迁移数据统计
- 未解决问题
- 后续 EPIC-02 需要的接口点

---

# 25. Definition of Done

只有全部满足才可认定 EPIC-01 完成：

- [ ] 核心业务表不再以 `data_json` 作为唯一权威数据
- [ ] 金额使用整数分
- [ ] 高频查询字段使用真实列和索引
- [ ] 外键和唯一约束有效
- [ ] Migration 有版本和 checksum
- [ ] Migration 前自动备份
- [ ] 备份通过 integrity check
- [ ] 迁移失败完整回滚
- [ ] 旧数据库可成功迁移
- [ ] 迁移前后记录数一致
- [ ] 财务金额差值为 0 cents
- [ ] 旧 `/api/state` 仍能支持当前 UI
- [ ] 旧 JSON 备份仍能恢复
- [ ] 核心旧表不再继续写入
- [ ] 前端测试通过
- [ ] Rust 测试通过
- [ ] 构建通过
- [ ] 文档完整
- [ ] 无静默丢数据
- [ ] 无为了通过测试而清空数据库的代码

---

# 26. Agent 执行提示词

下面内容可以直接复制给 Codex、Claude Code 或其他编码 Agent。

```text
你现在负责完成 LifeTrace 仓库的 EPIC-01：数据层重构。

仓库：
zhouxingxing1279/LifeTrace

当前技术栈：
Tauri 2 + React/Vite + Rust/Axum + rusqlite + SQLite。

你的目标不是简单新增几张表，而是把当前大量依赖
(id, data_json, updated_at)
的核心业务存储迁移为真实列结构，同时保证旧数据不丢失、现有 UI 不失效、迁移失败可恢复。

一、开始前必须完成只读审计

必须阅读：
- src-tauri/src/server.rs
- src-tauri/src/server/state.rs
- src-tauri/src/server/migration.rs
- src-tauri/src/server/notes.rs
- src-tauri/src/server/english.rs
- src-tauri/src/server/imports.rs
- src-tauri/src/server/xunji.rs
- src/types/index.ts
- src/types/english.ts
- src/stores/useLifeStore.ts
- src/services/

搜索整个仓库中的：
- CREATE TABLE
- ALTER TABLE
- data_json
- ensure_schema
- Connection::open
- INSERT INTO
- UPDATE
- DELETE FROM
- /api/state
- 备份和恢复逻辑

先生成：
docs/epic-01/current-schema-audit.md

审计文档必须列出：
- 当前所有 SQLite 表
- 每张表的创建位置
- 当前字段
- JSON 内真实业务字段
- API 和前端读写入口
- 当前 Migration 路径
- 风险点
- 旧数据库和备份格式

完成审计前不要开始大规模改代码。

二、硬性约束

1. 不得删除或覆盖用户原数据库。
2. 任何破坏性 Migration 前必须创建 SQLite 一致性备份。
3. 不能在 WAL 模式下只复制主 db 文件。
4. 使用 SQLite Backup API 或 VACUUM INTO。
5. 备份后执行 PRAGMA integrity_check。
6. 迁移失败必须回滚。
7. 迁移失败不得写 schema_migrations 成功记录。
8. 不得静默跳过无法解析的记录。
9. 财务金额必须使用 INTEGER amount_cents。
10. Tiptap、第三方原始响应、OCR 原始数据可以继续使用 JSON。
11. 旧 ID 原样保留。
12. 不提前实现云同步、账号、Android、AI 或邮件系统。
13. 不得为了通过测试清空数据库。
14. 不得只创建表而不迁移真实旧数据。
15. 不得让现有 UI 直接失效。
16. 所有 SQL 使用参数绑定。
17. 所有外键测试在 foreign_keys=ON 下运行。
18. 核心表迁移后不能继续写旧 data_json 表。

三、实现 Migration 框架

新增统一 database 模块，至少包含：
- connection
- backup
- migration_runner
- validation
- migrations
- legacy parser
- repositories

建立：
- schema_migrations
- migration_runs
- migration_issues

Migration 必须有：
- version
- name
- checksum
- up()
- MigrationReport

启动顺序改为：
打开数据库
→ 设置 PRAGMA
→ 执行版本化 Migration
→ 初始化非核心模块
→ 启动服务

核心模块的 ensure_schema 不再自行创建业务表。

四、分阶段迁移

按以下顺序，每一阶段独立测试和提交：

阶段 1：Migration 和备份框架
阶段 2：财务
阶段 3：习惯和每日复盘
阶段 4：笔记
阶段 5：英语
阶段 6：训记和训练摘要
阶段 7：清理旧写路径和文档

不要一次性修改所有模块后再测试。

五、财务要求

建立：
- finance_accounts
- transaction_categories
- transactions
- transaction_evidence

金额：
旧 number → ROUND(number * 100) → amount_cents

transactions 真实列至少包括：
- id
- user_id
- transaction_type
- amount_cents
- currency
- account_id
- to_account_id
- category_id
- counterparty
- merchant
- item
- note
- occurred_at
- local_date
- status
- source_type
- external_transaction_id
- legacy_category_name
- legacy_account_name
- raw_json
- created_at
- updated_at
- deleted_at
- version
- modified_by_device

必须校验：
- 交易数量一致
- 支出总额一致
- 收入总额一致
- 转账总额一致
- 每月汇总一致
- 金额差值为 0 cents
- 外键无异常

六、习惯和复盘要求

建立规范化：
- activities
- activity_logs
- daily_reviews

activity_logs 必须有：
- activity_id 外键
- log_date 索引

daily_reviews 必须对：
(user_id, review_date)
建立非删除记录唯一约束。

遇到同日多条旧复盘不得删除，记录 migration issue 并保留原始数据。

七、笔记要求

建立：
- note_folders
- notes
- note_tags
- note_tag_relations
- note_relations
- note_revisions

笔记正文继续保留：
- content_json
- content_html
- content_text
- content_markdown

元数据全部真实列化。

列表查询不能读取完整 content_json。
详情查询才读取正文。

FTS5 可用则建立 FTS，不可用继续使用参数化 LIKE。
FTS 失败不能导致主 Migration 失败。

八、英语要求

以当前 english.rs 和 src/types/english.ts 为准，规范化：
- english_articles
- english_learning_records
- english_highlights
- english_notes
- english_vocabulary
- vocabulary_occurrences
- vocabulary_review_state
- english_ai_analysis

vocabulary_review_state 只预留 FSRS 字段，不实现完整 FSRS。

九、训记要求

LifeTrace 不建设完整健身模块。

只规范：
- workout_imports
- workouts
- workout_exercises
- workout_sets
- training_notes

保留：
- source
- share_url
- parser
- parser_version
- raw_json
- source_id

十、兼容当前前端

保留现有 /api/state 协议作为兼容层。

读取：
真实列 → Rust Model → 旧 camelCase DTO

写入：
旧 DTO → Validation → Repository → 新表

财务返回前端时：
amount_cents / 100.0 → amount

但数据库内部永远使用 amount_cents。

不得继续把整个 DTO 写回 data_json。

十一、备份恢复

旧 JSON 备份必须继续可导入。

增加：
- format
- schemaVersion
- createdAt

导入时识别旧版和新版，统一转换后写入规范化表。
恢复前必须先创建数据库备份。

十二、测试

必须增加：
- 金额转换测试
- 时间解析测试
- Migration 幂等测试
- Migration 失败回滚测试
- 备份 integrity check 测试
- 空数据库测试
- 当前 JSON 数据库升级测试
- 旧 D1 数据库导入测试
- 重复复盘测试
- 非法金额测试
- 孤立关系测试
- /api/state 兼容测试
- JSON 备份恢复测试

运行：
npm.cmd test
npm.cmd run test:rust
npm.cmd run build

十三、提交要求

每个阶段单独提交，提交信息清晰，例如：
- feat(db): add migration and backup framework
- feat(db): normalize finance schema
- feat(db): migrate habits and reviews
- feat(db): normalize notes schema
- feat(db): normalize english schema
- feat(db): normalize workout imports
- test(db): add migration fixtures and validation
- docs(db): document epic-01 schema migration

十四、最终报告

完成后输出：
1. 实际修改文件列表
2. Migration 版本列表
3. 旧表到新表映射
4. 每张表迁移数量
5. 财务迁移前后金额对比
6. migration issue 列表
7. 测试结果
8. 构建结果
9. 未解决问题
10. 后续 EPIC-02 需要的接口点

在所有 Definition of Done 满足前，不要声称 EPIC-01 已完成。
```
