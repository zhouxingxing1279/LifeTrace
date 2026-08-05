
# LifeTrace EPIC-02：统一领域模型与同步协议——Agent 具体实施方案

> 目标仓库：`zhouxingxing1279/LifeTrace`  
> 当前技术栈：Tauri 2 + React/Vite + Rust/Axum + rusqlite + SQLite  
> 前置任务：EPIC-01 数据层重构的目标模型、Migration 和真实列表结构已经确定；实际实施时必须以仓库当前状态为准。  
> 本 Epic 目标：建立 Windows、未来 Android、Web 和云端共同使用的**领域数据契约与同步协议 v1**。  
> 核心原则：**契约优先、服务端排序、幂等提交、显式冲突、删除墓碑、向前兼容、协议与实现解耦。**

---

# 1. EPIC-02 的最终交付目标

EPIC-02 完成后，LifeTrace 应拥有一套独立于桌面 UI、SQLite 实现和未来云端实现的公共契约：

```text
LifeTrace Desktop
LifeTrace Android
LifeTrace Web
LifeTrace Cloud
        ↓
统一领域模型
统一实体类型名称
统一 JSON 字段
统一同步请求/响应
统一版本协商
统一错误码
统一冲突语义
```

本 Epic 必须交付：

1. 公共 Rust 契约 crate
2. 领域实体公共 DTO
3. 稳定的 `entity_type` 注册表
4. 同步协议 v1：
   - Push
   - Pull
   - Snapshot
   - Tombstone
   - Conflict
   - Capabilities
5. 协议版本、Schema 版本和客户端版本兼容规则
6. JSON Schema / TypeScript 生成物
7. OpenAPI 协议文档
8. Golden Fixture 和契约测试
9. 一个仅供测试的内存同步参考实现
10. 桌面端与公共契约的适配层
11. 后续 EPIC-03、EPIC-05 可直接使用的接口边界

---

# 2. 当前仓库基线

Agent 开始实施前必须实际读取：

```text
README.md
package.json
src/types/index.ts
src/types/english.ts

src-tauri/Cargo.toml
src-tauri/src/lib.rs
src-tauri/src/server.rs
src-tauri/src/server/state.rs
src-tauri/src/server/migration.rs
src-tauri/src/server/notes.rs
src-tauri/src/server/english.rs
src-tauri/src/server/imports.rs
src-tauri/src/server/xunji.rs

EPIC-01 产生的：
src-tauri/src/database/**
docs/epic-01/**
```

当前已知情况：

- 当前仓库主要是单个 Tauri 桌面应用
- Rust 后端和 React 前端在同一仓库
- 当前 TypeScript 领域类型主要位于 `src/types`
- 当前 Rust 服务未形成独立共享契约 crate
- 当前没有正式云端同步服务
- 当前没有成熟的 Push/Pull、cursor、change log 或冲突协议
- 当前数据库以 SQLite 为主
- 当前桌面应用启动本地 Axum 服务
- 未来云端仍计划使用 Rust/Axum

必须以 Agent 实际读取到的代码为最终依据，不得根据本文件猜测不存在的实现。

---

# 3. EPIC-02 与其他 Epic 的边界

## 3.1 本 Epic 必须实现

- 公共领域 DTO
- 公共实体类型注册表
- 同步 Wire Contract
- 协议版本协商
- Push/Pull/Snapshot 数据结构
- 墓碑结构
- 冲突结构
- 错误码
- 批量、分页和幂等规则
- JSON Schema
- TypeScript 类型生成
- OpenAPI 文档
- 契约测试
- 内存参考实现
- 桌面端 DTO 适配示例
- 协议 ADR 和说明文档

## 3.2 本 Epic 不实现

以下内容属于后续 Epic：

### EPIC-03 云端后端

- PostgreSQL 实体表
- 正式云端 Axum API
- 正式认证
- 正式服务器 change log
- 正式生产 cursor
- 云端部署

### EPIC-04 账号与设备

- 注册
- 登录
- Refresh Token
- 设备注册
- Token 吊销
- 账号权限

### EPIC-05 客户端同步核心

- SQLite sync outbox
- 后台推送 Worker
- 拉取 Worker
- 网络重试
- 同步调度
- UI 同步状态
- 自动冲突处理界面

### EPIC-12 文件对象存储

- 二进制分片上传
- 预签名 URL
- 对象存储
- 下载缓存

EPIC-02 可以定义这些模块未来使用的契约，但不得提前实现生产系统。

---

# 4. 核心架构决策

## 4.1 权威契约来源

推荐新增：

```text
crates/lifetrace-contracts/
```

以 Rust 类型作为领域和同步 Wire Contract 的权威来源。

原因：

- 桌面后端是 Rust
- 未来云端后端计划使用 Rust
- Rust 类型可共享
- 可通过 `serde` 序列化
- 可通过 `schemars` 生成 JSON Schema
- 可通过 `ts-rs` 生成 TypeScript 类型
- 可通过 `utoipa` 或独立生成器输出 OpenAPI
- Android 后续可依据 OpenAPI 生成 Kotlin DTO

不得把当前 React TypeScript 接口继续作为唯一权威契约。

## 4.2 数据库模型与 Wire DTO 分离

必须区分：

```text
Database Row
Domain Model
Wire DTO
Frontend View Model
```

禁止：

```text
SQLite Row
=
API JSON
=
前端状态
=
同步协议
```

推荐：

```text
SQLite Row
→ Repository
→ Domain Model
→ Sync DTO / API DTO
```

数据库可以使用 snake_case，Wire JSON 使用 camelCase。

## 4.3 同步 v1 使用完整实体快照

同步 Change v1 使用：

```text
operation = upsert
payload = 完整实体同步快照
```

不使用字段级 JSON Patch。

原因：

- 第一版实现更稳定
- 客户端更容易重试
- 不依赖补丁顺序
- 协议更容易跨 Rust、TypeScript、Kotlin
- 减少局部更新与 Schema 演进风险

字段级合并可以后续增加，不在 v1 实现。

## 4.4 服务端负责全局排序

禁止使用：

```text
client updated_at
客户端本地时间
设备系统时间
```

决定同步先后。

服务端必须分配：

```text
server_cursor
server_version
server_modified_at
```

客户端时间只能作为展示和诊断信息。

## 4.5 本地版本与服务端版本分离

EPIC-01 中已有或计划存在本地 `version`。该字段不得直接承担服务端同步版本职责。

统一定义：

```text
local version
→ 本地实体修订号，用于本地乐观并发或历史

serverVersion
→ 云端实体权威版本，用于同步冲突判断

baseServerVersion
→ 客户端生成变更时已知的服务端版本
```

本地新实体：

```text
serverVersion = null
baseServerVersion = "0"
```

离线修改：

- 可以增加本地 `version`
- 不得伪造新的 `serverVersion`
- Outbox 在 EPIC-05 中保存 `baseServerVersion`

---

# 5. 推荐目录结构

```text
crates/
└── lifetrace-contracts/
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs
    │   ├── ids.rs
    │   ├── time.rs
    │   ├── money.rs
    │   ├── common.rs
    │   ├── registry.rs
    │   ├── error.rs
    │   │
    │   ├── domain/
    │   │   ├── mod.rs
    │   │   ├── user.rs
    │   │   ├── device.rs
    │   │   ├── finance.rs
    │   │   ├── habits.rs
    │   │   ├── reviews.rs
    │   │   ├── notes.rs
    │   │   ├── english.rs
    │   │   ├── workouts.rs
    │   │   ├── files.rs
    │   │   ├── preferences.rs
    │   │   └── links.rs
    │   │
    │   └── sync/
    │       ├── mod.rs
    │       └── v1/
    │           ├── mod.rs
    │           ├── client.rs
    │           ├── change.rs
    │           ├── push.rs
    │           ├── pull.rs
    │           ├── snapshot.rs
    │           ├── conflict.rs
    │           ├── capability.rs
    │           └── tombstone.rs
    │
    ├── tests/
    │   ├── round_trip.rs
    │   ├── compatibility.rs
    │   ├── schemas.rs
    │   ├── golden.rs
    │   └── testkit.rs
    │
    └── fixtures/
        └── sync-v1/
            ├── push-request.json
            ├── push-success.json
            ├── push-conflict.json
            ├── pull-response.json
            ├── delete-tombstone.json
            ├── snapshot-page.json
            └── error-response.json

tools/
└── contract-exporter/
    ├── Cargo.toml
    └── src/main.rs

contracts/
├── json-schema/
├── openapi/
│   └── lifetrace-sync-v1.json
└── typescript/
    └── lifetrace-contracts.generated.ts

docs/epic-02/
├── current-contract-audit.md
├── domain-model.md
├── entity-registry.md
├── sync-protocol-v1.md
├── compatibility-policy.md
├── error-codes.md
├── integration-guide.md
└── adr/
    ├── 001-contract-source.md
    ├── 002-full-snapshot-changes.md
    ├── 003-server-cursor.md
    ├── 004-conflict-policy.md
    └── 005-versioning.md
```

允许根据当前仓库结构调整，但不得把所有类型继续放入一个超大文件。

---

# 6. 依赖建议

`crates/lifetrace-contracts/Cargo.toml` 建议使用：

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
schemars = { version = "1", features = ["chrono04", "uuid1"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
ts-rs = { version = "11", features = ["chrono-impl", "uuid-impl"] }
uuid = { version = "1", features = ["serde", "v4"] }
```

如 Agent 选择 `utoipa` 生成 OpenAPI，需要说明原因并控制依赖范围。

要求：

- 契约 crate 不依赖 Axum
- 契约 crate 不依赖 rusqlite
- 契约 crate 不依赖 Tauri
- 契约 crate 不依赖 React
- 不在契约 crate 放业务数据库访问代码
- 不在契约 crate 放网络客户端实现

---

# 7. 基础值对象

## 7.1 ID

Wire 层所有 ID：

```text
JSON string
UUID 格式
```

Rust 推荐新类型：

```rust
pub struct UserId(pub Uuid);
pub struct DeviceId(pub Uuid);
pub struct EntityId(pub Uuid);
pub struct ChangeId(pub Uuid);
pub struct RequestId(pub Uuid);
pub struct ConflictId(pub Uuid);
```

要求：

- 已有旧 ID 若是合法 UUID则保留
- 如果历史 ID 不是 UUID，不得在同步时静默更换
- 对历史非 UUID ID，应在 EPIC-01 或兼容层建立稳定 UUID 映射
- 同一旧 ID 每次映射结果必须一致
- 新实体优先继续使用 UUID v4
- 是否改用 UUID v7属于后续 ADR，不在本 Epic 强制

## 7.2 金额

```rust
pub struct MoneyAmount {
    pub amount_cents: i64,
    pub currency: CurrencyCode,
}
```

Wire：

```json
{
  "amountCents": 3250,
  "currency": "CNY"
}
```

禁止：

```json
{
  "amount": 32.5
}
```

进入同步协议。

## 7.3 时间

时间点：

```text
RFC3339 UTC
2026-08-04T15:30:00Z
```

本地自然日：

```text
YYYY-MM-DD
2026-08-04
```

要求：

- Wire 时间带时区
- 服务端保存标准 UTC
- `localDate` 单独传递
- 不通过 `localDate` 推导绝对时间
- 不通过客户端时间决定变更顺序

## 7.4 服务端版本

为避免 JavaScript 安全整数问题：

```text
serverVersion
baseServerVersion
serverCursor
```

在线路 JSON 中使用字符串：

```json
{
  "serverVersion": "42",
  "baseServerVersion": "41",
  "cursor": "10591"
}
```

Rust 定义验证型 newtype，不直接暴露裸 `u64` JSON number。

---

# 8. 公共实体元数据

定义：

```rust
pub struct EntityMeta {
    pub id: EntityId,
    pub user_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub local_version: u64,
    pub server_version: Option<ServerVersion>,
}
```

说明：

- `localVersion` 用于本地实体修订
- `serverVersion` 来自云端确认
- `deletedAt` 表示业务软删除
- 同步拉取的删除事件仍需 Tombstone
- `modifiedByDevice` 可以作为可选审计字段
- 不把数据库内部 rowid 传给其他客户端

---

# 9. 实体所有权与同步范围

必须建立实体注册表，明确每类实体的同步属性。

```rust
pub struct EntityDescriptor {
    pub entity_type: &'static str,
    pub schema_version: u32,
    pub ownership: EntityOwnership,
    pub sync_mode: SyncMode,
    pub conflict_mode: ConflictMode,
    pub contains_file_references: bool,
}
```

## 9.1 Ownership

```text
user_owned
server_managed
shared_catalog
device_local
secret_local_only
```

## 9.2 SyncMode

```text
bidirectional
server_to_client
client_to_server
not_synced
```

## 9.3 初始实体注册表

### 用户和设备

```text
identity.user
identity.device
```

说明：

- 用户由服务端管理
- 设备注册由 EPIC-04 实现
- 本 Epic 只定义 DTO

### 财务

```text
finance.account
finance.category
finance.transaction
finance.transaction_evidence
```

### 习惯与复盘

```text
habit.activity
habit.log
review.daily
```

### 笔记

```text
note.folder
note.note
note.tag
note.tag_relation
note.relation
note.revision
```

### 英语

```text
english.article
english.learning_record
english.highlight
english.note
english.vocabulary
english.vocabulary_occurrence
english.vocabulary_review_state
```

建议：

- `english.article` 作为 `shared_catalog` 或 `server_to_client`
- 用户阅读、高亮、生词等作为 `user_owned`

### 训记和训练摘要

```text
workout.import
workout.workout
workout.exercise
workout.set
workout.training_note
```

### 文件和关联

```text
file.metadata
entity.link
user.preference
```

## 9.4 永不进入普通同步 Payload

```text
API Key
密码
邮箱授权码
Refresh Token
私钥
本地证书私钥
数据库文件
缓存
缩略图缓存
SQLite WAL/SHM
模型供应商密钥
```

这些必须标记：

```text
secret_local_only
```

---

# 10. 领域 DTO 要求

领域 DTO 必须与 EPIC-01 的规范化模型对齐。

## 10.1 财务

至少：

```rust
FinanceAccount
TransactionCategory
Transaction
TransactionEvidence
```

Transaction Wire 字段至少包括：

```text
meta
transactionType
amountCents
currency
accountId
toAccountId
categoryId
counterparty
merchant
item
note
occurredAt
localDate
status
sourceType
externalTransactionId
```

不把：

```text
legacyAccountName
legacyCategoryName
raw import secrets
```

默认放进跨端 DTO。

兼容字段只有明确需要时才进入 `metadata`。

## 10.2 习惯和复盘

至少：

```text
Activity
ActivityLog
DailyReview
```

`ActivityLog`：

- `activityId`
- `logDate`
- `value`
- `status`
- `note`
- `metadata`

## 10.3 笔记

至少：

```text
NoteFolder
Note
NoteTag
NoteTagRelation
NoteRelation
NoteRevision
```

正文：

```text
contentJson
contentHtml
contentText
contentMarkdown
```

必须明确：

- 列表 DTO 可不携带完整正文
- 同步 DTO 必须携带权威正文快照
- 附件只传文件 ID，不传文件二进制
- Relation 使用稳定 entity type

## 10.4 英语

以当前实际代码为准统一：

```text
EnglishArticle
EnglishLearningRecord
EnglishHighlight
EnglishNote
EnglishVocabulary
VocabularyOccurrence
VocabularyReviewState
```

## 10.5 训记

只传训练摘要和解析结果：

```text
WorkoutImport
Workout
WorkoutExercise
WorkoutSet
TrainingNote
```

不扩展成完整 LifeTrace 健身平台。

## 10.6 文件元数据

定义：

```rust
pub struct FileMetadata {
    pub meta: EntityMeta,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub storage_state: FileStorageState,
    pub created_by_device: Option<DeviceId>,
}
```

本 Epic 不传：

```text
object storage key
预签名 URL
本地绝对路径
```

二进制传输由 EPIC-12 实现。

## 10.7 跨实体关联

```rust
pub struct EntityRef {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
}

pub struct EntityLink {
    pub meta: EntityMeta,
    pub source: EntityRef,
    pub target: EntityRef,
    pub relation_type: String,
    pub metadata: serde_json::Value,
}
```

`relationType` 使用稳定、可扩展的命名字符串，例如：

```text
created_from
references
attachment
summary_of
belongs_to
evidence_for
```

不得把目标实体表名直接硬编码到业务逻辑。

---

# 11. 同步客户端信息

定义：

```rust
pub struct SyncClientInfo {
    pub app_id: AppId,
    pub client_version: String,
    pub platform: ClientPlatform,
    pub protocol_version: u32,
    pub schema_version: u32,
    pub device_id: DeviceId,
}
```

## appId 初始值

```text
lifetrace-desktop
lifetrace-finance-android
lifetrace-notes-android
lifetrace-english-android
lifetrace-habits-android
lifetrace-web
```

## platform

```text
windows
android
web
```

扩展时不得破坏旧客户端反序列化。

---

# 12. Sync Change v1

```rust
pub struct SyncChangeV1 {
    pub change_id: ChangeId,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub operation: ChangeOperation,
    pub base_server_version: ServerVersion,
    pub entity_schema_version: u32,
    pub client_modified_at: DateTime<Utc>,
    pub payload: Option<serde_json::Value>,
    pub atomic_group_id: Option<Uuid>,
    pub dependencies: Vec<EntityRef>,
}
```

## operation

```text
upsert
delete
```

## upsert

- `payload` 必须存在
- payload 必须是完整实体快照
- payload ID 必须与 `entityId` 一致
- payload 类型必须与 `entityType` 一致
- payload Schema 必须通过校验

## delete

- `payload` 默认为空
- `baseServerVersion` 必须为客户端已知版本
- 删除后服务器保留 Tombstone
- 删除不能直接从 change log 消失

## clientModifiedAt

仅用于：

- 审计
- 展示
- 诊断时钟漂移

不得用于：

- 决定全局顺序
- 自动解决冲突
- 替代 cursor

---

# 13. Push 协议

## 13.1 Endpoint Contract

```text
POST /api/v1/sync/push
```

本 Epic 只定义 OpenAPI 和 DTO，不实现生产服务。

## 13.2 PushRequest

```rust
pub struct PushRequestV1 {
    pub request_id: RequestId,
    pub client: SyncClientInfo,
    pub changes: Vec<SyncChangeV1>,
}
```

## 13.3 批量规则

建议默认：

```text
默认批量：100
最大批量：500
最大请求体：2 MiB
单个 atomic group 最大：50
```

具体限制必须写入 capabilities，不得只写死在客户端。

## 13.4 PushResponse

```rust
pub struct PushResponseV1 {
    pub request_id: RequestId,
    pub server_time: DateTime<Utc>,
    pub results: Vec<PushChangeResultV1>,
    pub latest_cursor: Cursor,
}
```

单项结果：

```text
accepted
duplicate
conflict
rejected
```

### accepted

返回：

```text
changeId
entityType
entityId
serverVersion
cursor
serverModifiedAt
```

### duplicate

同一 `changeId` 重试：

- 不重复修改实体
- 返回第一次处理结果
- 结果语义与第一次一致

### conflict

返回：

```text
conflictId
changeId
entityType
entityId
clientBaseServerVersion
currentServerVersion
serverEntity
serverDeleted
reason
```

### rejected

返回稳定错误码和字段错误。

## 13.5 幂等规则

服务端必须以：

```text
user_id + change_id
```

建立唯一约束。

重复 `changeId`：

### Payload 相同

返回首次结果：

```text
status=duplicate
```

### Payload 不同

返回：

```text
SYNC_CHANGE_ID_REUSE
```

禁止把同一个 Change ID 用于不同内容。

`requestId` 用于请求追踪，不替代 `changeId` 幂等。

---

# 14. Atomic Group

一个本地业务事务可能生成多个相关实体变化，例如：

```text
交易
+
交易证据
+
账户相关记录
```

定义可选：

```text
atomicGroupId
```

规则：

- 相同 group 必须出现在同一 Push Request
- 服务端对同组全部成功或全部失败
- 任意一项冲突，整个组失败
- group 结果使用 `SYNC_ATOMIC_GROUP_FAILED`
- 不同 group 之间可 best-effort 处理
- 未设置 group 的 Change 独立处理

本 Epic 在内存参考实现中验证该语义。

---

# 15. Pull 协议

## 15.1 Endpoint

```text
POST /api/v1/sync/pull
```

使用 POST 是为了携带客户端信息、实体过滤和扩展参数。

## 15.2 PullRequest

```rust
pub struct PullRequestV1 {
    pub request_id: RequestId,
    pub client: SyncClientInfo,
    pub after_cursor: Option<Cursor>,
    pub limit: u32,
    pub entity_types: Option<Vec<EntityType>>,
}
```

## 15.3 PullResponse

```rust
pub struct PullResponseV1 {
    pub request_id: RequestId,
    pub server_time: DateTime<Utc>,
    pub changes: Vec<ServerChangeV1>,
    pub next_cursor: Cursor,
    pub has_more: bool,
}
```

`ServerChangeV1` 至少：

```text
cursor
entityType
entityId
operation
serverVersion
serverModifiedAt
payload
tombstone
originDeviceId
```

## 15.4 排序规则

Pull Change 必须：

```text
按服务端 cursor 严格升序
```

客户端：

- 按顺序应用
- 当前批次全部成功后保存 `nextCursor`
- 中途失败不得提前推进 cursor
- 不得根据 `updatedAt` 重新排序

## 15.5 Cursor

Cursor 在线路层是 opaque string：

```json
{
  "afterCursor": "10591"
}
```

客户端不得：

- 做数值加减
- 猜测下一 cursor
- 比较不同服务器的 cursor
- 用客户端时间替代 cursor

---

# 16. Tombstone

定义：

```rust
pub struct TombstoneV1 {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub deleted_at: DateTime<Utc>,
    pub server_version: ServerVersion,
    pub deleted_by_device: Option<DeviceId>,
}
```

规则：

- 删除事件必须进入 change log
- 客户端 Pull 后进行本地软删除
- 不立即清理本地关系和附件
- Tombstone 保留周期由服务器 capabilities 声明
- cursor 超出保留周期时要求 Snapshot
- 被删除实体重新创建必须使用新 ID
- 不得通过旧 ID 隐式“复活”

如需要恢复软删除，使用：

```text
upsert existing entity
baseServerVersion = tombstone current version
```

并由业务策略确认。

---

# 17. Snapshot 协议

## 17.1 使用场景

- 新设备首次同步
- cursor 不存在
- cursor 已过期
- 服务端要求重建
- 客户端主动完整校验

## 17.2 Endpoint

```text
POST /api/v1/sync/snapshot
```

## 17.3 SnapshotRequest

```rust
pub struct SnapshotRequestV1 {
    pub request_id: RequestId,
    pub client: SyncClientInfo,
    pub snapshot_id: Option<Uuid>,
    pub page_token: Option<String>,
    pub entity_types: Option<Vec<EntityType>>,
    pub page_size: u32,
}
```

## 17.4 SnapshotResponse

```rust
pub struct SnapshotResponseV1 {
    pub request_id: RequestId,
    pub snapshot_id: Uuid,
    pub snapshot_cursor: Cursor,
    pub items: Vec<EntitySnapshotV1>,
    pub next_page_token: Option<String>,
    pub completed: bool,
    pub server_time: DateTime<Utc>,
}
```

规则：

- 第一次响应固定 `snapshotId`
- 同一 snapshot 所有页面对应一致视图
- 完成后客户端将 cursor 设置为 `snapshotCursor`
- 然后继续 Pull：
  - `afterCursor=snapshotCursor`
- Snapshot 分页不得因并发修改漏数据
- 新变更通过 snapshotCursor 之后的 Pull 获取

---

# 18. Capabilities 与版本协商

## 18.1 Endpoint

```text
GET /api/v1/sync/capabilities
```

## 18.2 CapabilitiesResponse

至少包含：

```text
protocolVersion
supportedProtocolVersions
schemaVersion
minimumSchemaVersion
minimumClientVersions
maximumPushBatchSize
maximumPullBatchSize
maximumRequestBytes
maximumSnapshotPageSize
tombstoneRetentionDays
supportedEntityTypes
serverTime
```

## 18.3 请求头

建议正式 API 使用：

```text
X-LifeTrace-App-Id
X-LifeTrace-Client-Version
X-LifeTrace-Protocol-Version
X-LifeTrace-Schema-Version
X-LifeTrace-Device-Id
X-Request-Id
```

Body 仍包含 `client`，用于契约完整性和测试。

生产实现可在服务端校验 Header 与 Body 一致。

---

# 19. 协议与 Schema 版本

必须明确区分：

```text
protocolVersion
→ Push/Pull/Snapshot 线路协议版本

schemaVersion
→ 当前整体领域契约版本

entitySchemaVersion
→ 单个实体 payload 版本

clientVersion
→ App 发布版本

appId
→ 客户端产品标识
```

## 19.1 v1 兼容规则

允许不升级 protocolVersion：

- 新增可选字段
- 新增客户端可忽略的响应元数据
- 新增新 entity type
- 新增错误详情字段
- 放宽限制

必须升级 protocolVersion：

- 重命名必填字段
- 改变字段含义
- 改变 Push/Pull 状态机
- 改变 cursor 语义
- 改变幂等规则
- 删除既有状态
- 改变冲突行为

## 19.2 未知字段

客户端必须忽略未知 JSON 字段。

## 19.3 未知枚举

不得让新增枚举值导致整个同步批次无法解析。

实现方案之一：

```rust
KnownVariant
Unknown(String)
```

或使用字符串 newtype。

不建议所有 Wire Enum 使用无法承载未知值的封闭 Rust enum。

## 19.4 客户端太旧

返回：

```text
HTTP 426 Upgrade Required
LIFETRACE_CLIENT_TOO_OLD
```

响应包含：

```text
minimumClientVersion
minimumProtocolVersion
minimumSchemaVersion
```

---

# 20. 冲突语义

## 20.1 冲突条件

服务端当前：

```text
serverVersion = 8
```

客户端 Change：

```text
baseServerVersion = 7
```

则：

```text
conflict
```

不得自动使用客户端 `updatedAt` 覆盖服务器。

## 20.2 v1 默认策略

```text
no automatic last-write-wins
```

冲突返回服务器当前实体，由客户端或用户决定。

## 20.3 解决方式

后续客户端支持：

```text
keep_server
keep_local
manual_merge
```

### keep_server

- 丢弃本地待同步变更
- 应用服务器实体

### keep_local

- 使用服务器最新实体版本作为新的 `baseServerVersion`
- 生成新的 `changeId`
- 重新 Push
- 必须保留冲突审计

### manual_merge

- 用户或业务层生成合并实体
- 使用新的 `changeId`
- 基于服务器最新版本 Push

## 20.4 删除冲突

以下情况必须显式处理：

```text
客户端修改，服务器已删除
客户端删除，服务器已修改
双方都删除
```

双方都删除可视为幂等成功，但仍返回当前 tombstone version。

---

# 21. 错误模型

统一：

```rust
pub struct ApiErrorV1 {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: Option<RequestId>,
    pub retryable: bool,
    pub field_errors: Vec<FieldError>,
    pub details: Option<serde_json::Value>,
}
```

## 21.1 初始错误码

### 协议和版本

```text
LIFETRACE_PROTOCOL_UNSUPPORTED
LIFETRACE_SCHEMA_UNSUPPORTED
LIFETRACE_CLIENT_TOO_OLD
LIFETRACE_APP_ID_UNSUPPORTED
```

### 认证和设备

```text
LIFETRACE_AUTH_REQUIRED
LIFETRACE_AUTH_INVALID
LIFETRACE_DEVICE_NOT_REGISTERED
LIFETRACE_DEVICE_REVOKED
```

### 请求

```text
LIFETRACE_INVALID_REQUEST
LIFETRACE_BATCH_TOO_LARGE
LIFETRACE_PAYLOAD_TOO_LARGE
LIFETRACE_UNKNOWN_ENTITY_TYPE
LIFETRACE_INVALID_ENTITY_PAYLOAD
LIFETRACE_DEPENDENCY_MISSING
```

### 同步

```text
LIFETRACE_CHANGE_ID_REUSE
LIFETRACE_BASE_VERSION_MISMATCH
LIFETRACE_CURSOR_INVALID
LIFETRACE_CURSOR_EXPIRED
LIFETRACE_SNAPSHOT_REQUIRED
LIFETRACE_ATOMIC_GROUP_FAILED
```

### 服务

```text
LIFETRACE_RATE_LIMITED
LIFETRACE_TEMPORARILY_UNAVAILABLE
LIFETRACE_INTERNAL_ERROR
```

错误码一旦发布不得改变含义。

---

# 22. HTTP 状态规则

建议：

```text
200
→ 请求成功处理，单项 Change 可 accepted/conflict/rejected

400
→ 请求结构错误

401
→ 未认证

403
→ 设备或用户无权限

413
→ 请求体过大

426
→ 客户端或协议版本过旧

429
→ 限流

500
→ 服务内部错误

503
→ 临时不可用
```

Push 中的业务冲突不必让整个 HTTP 请求返回 409，因为同一批次可能部分成功。

单项冲突放入 `results`。

---

# 23. 内存同步参考实现

为了验证协议，而不是提前实现生产服务器，新增测试专用：

```text
crates/lifetrace-contracts/src/sync/testkit/
```

或单独 crate：

```text
crates/lifetrace-sync-testkit/
```

参考实现维护：

```text
entities:
Map<(user_id, entity_type, entity_id), StoredEntity>

processed_changes:
Map<(user_id, change_id), StoredChangeResult>

change_log:
Vec<ServerChange>

next_cursor:
u64
```

必须验证：

1. Create 成功
2. Update base version 正确
3. Update base version 冲突
4. 重复 changeId 幂等
5. changeId 重用不同 payload 被拒绝
6. Delete 生成 Tombstone
7. 双方删除幂等
8. Pull 按 cursor 排序
9. Pull 分页无重复无遗漏
10. Snapshot cursor 一致
11. Snapshot 后 Pull 不漏并发变更
12. Atomic Group 全成或全败
13. 未知 entity type 被拒绝
14. 过期 cursor 要求 Snapshot

参考实现不得被生产桌面程序直接作为云端使用。

---

# 24. JSON Schema 和生成物

## 24.1 生成器

新增：

```text
tools/contract-exporter
```

命令：

```powershell
cargo run --manifest-path tools/contract-exporter/Cargo.toml
```

输出：

```text
contracts/json-schema/*.schema.json
contracts/openapi/lifetrace-sync-v1.json
contracts/typescript/lifetrace-contracts.generated.ts
```

要求：

- 输出稳定排序
- 同一代码重复执行无无意义 diff
- 生成文件带自动生成标记
- 禁止手工编辑 generated 文件
- CI 检查生成物是否过期

## 24.2 TypeScript

生成的 TypeScript 不立即强制替换所有前端现有接口。

第一阶段：

```text
src/types/contracts.ts
```

可以 re-export 或适配生成类型。

要求：

- 前端编译通过
- 公共 Wire DTO 不再手工重复定义
- UI View Model 仍可保留独立类型

## 24.3 Android

本 Epic 不创建 Android App。

但必须保证 OpenAPI 可用于后续 Kotlin 生成：

```text
OpenAPI Generator
Kotlin Serialization
```

并在文档中提供建议命令，不把生成的 Android 客户端提交进当前仓库。

---

# 25. OpenAPI 文档范围

至少定义：

```text
GET  /api/v1/sync/capabilities
POST /api/v1/sync/push
POST /api/v1/sync/pull
POST /api/v1/sync/snapshot
```

OpenAPI 必须包含：

- 请求头
- Request/Response DTO
- Error DTO
- 示例
- 批量限制说明
- cursor 说明
- 版本说明
- 幂等说明
- 冲突说明
- tombstone 说明

不得定义实际不存在的账号或文件上传接口。

---

# 26. 桌面端集成范围

EPIC-02 只完成契约接入，不实现真实同步 Worker。

建议在桌面端新增：

```text
src-tauri/src/contracts.rs
src-tauri/src/sync_adapter/
```

或在数据库 Repository 层实现：

```text
Domain Entity
↔ Contract DTO
```

必须至少完成一个端到端示例：

```text
SQLite Transaction
→ Domain Transaction
→ contracts::Transaction
→ SyncChangeV1
→ JSON
→ 反序列化
→ 验证字段一致
```

可选择：

- 财务交易
- 笔记

建议优先财务交易。

不得在 EPIC-02 中：

- 自动向网络发送
- 保存生产 outbox
- 建立后台定时同步
- 实现 Token 刷新

---

# 27. 审计文档

首先生成：

```text
docs/epic-02/current-contract-audit.md
```

必须记录：

- 当前 TypeScript 实体类型
- 当前 Rust 内部实体表示
- EPIC-01 数据表字段
- 当前 camelCase / snake_case 差异
- 金额表示差异
- 时间表示差异
- ID 类型差异
- nullable 差异
- 枚举差异
- 当前备份 DTO
- 当前 API DTO
- 未来同步实体范围
- 不同步数据清单
- 需要兼容的历史字段

审计完成前不得批量改类型。

---

# 28. ADR 要求

## ADR-001：公共契约权威来源

说明为何使用 Rust contract crate。

## ADR-002：v1 使用完整实体快照

说明为何暂不使用 JSON Patch。

## ADR-003：服务端 cursor 和 version

说明为何不依赖客户端时间。

## ADR-004：显式冲突

说明为何不默认 Last Write Wins。

## ADR-005：版本兼容

说明 protocol、schema、entity schema 和 client version 的区别。

---

# 29. 测试要求

## 29.1 序列化测试

每个公共 DTO：

- Rust → JSON
- JSON → Rust
- round trip 相等
- camelCase 字段正确
- 可选字段缺失可解析
- 未知字段可忽略

## 29.2 Golden Fixture

Fixture 一旦发布即视为 v1 兼容样本。

修改类型后必须验证旧 Fixture 仍可解析。

## 29.3 Schema Snapshot

- JSON Schema 生成稳定
- TypeScript 生成稳定
- OpenAPI 生成稳定
- CI 检测未提交变化

## 29.4 协议语义

至少覆盖：

```text
duplicate change
change id reuse
base version conflict
delete tombstone
cursor order
pull pagination
snapshot consistency
atomic group rollback
unknown entity
unsupported protocol
unsupported schema
```

## 29.5 TypeScript 编译

生成的 TypeScript 必须通过：

```powershell
npm.cmd run lint
```

## 29.6 Rust

执行：

```powershell
cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml
npm.cmd run test:rust
```

## 29.7 全仓库

执行：

```powershell
npm.cmd test
npm.cmd run test:rust
npm.cmd run build
```

---

# 30. CI 建议

新增脚本：

```json
{
  "scripts": {
    "contracts:generate": "cargo run --manifest-path tools/contract-exporter/Cargo.toml",
    "contracts:test": "cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml",
    "contracts:check": "npm run contracts:generate && git diff --exit-code -- contracts/"
  }
}
```

Windows 兼容要求：

- 不依赖 Bash
- 不依赖 jq
- 不依赖全局 OpenAPI Generator
- 生成器由 Rust 运行
- 文件使用 UTF-8
- 换行风格稳定

---

# 31. 分阶段实施步骤

## 阶段 0：契约审计

只读当前代码，生成：

```text
docs/epic-02/current-contract-audit.md
```

不修改公共模型。

## 阶段 1：契约 crate 基础

实现：

- crate
- ID newtype
- 时间类型
- 金额类型
- EntityMeta
- Error model
- 基础测试

## 阶段 2：领域 DTO

按顺序：

1. 财务
2. 习惯与复盘
3. 笔记
4. 英语
5. 训记
6. 文件与跨实体关联
7. 用户偏好
8. 用户和设备 DTO

每个领域单独测试。

## 阶段 3：Entity Registry

实现：

- 稳定 entity type
- schema version
- ownership
- sync mode
- conflict mode
- file reference 标记
- registry lookup
- 重复名称测试

## 阶段 4：Sync v1 DTO

实现：

- ClientInfo
- Change
- Push
- Pull
- Tombstone
- Snapshot
- Conflict
- Capabilities
- Error code

## 阶段 5：参考实现与协议测试

实现内存 testkit 和语义测试。

## 阶段 6：Schema/OpenAPI/TS 生成

实现 exporter、生成文件和 stale check。

## 阶段 7：桌面适配示例

至少完成财务交易：

```text
DB/Domain
↔ Contract
↔ Sync JSON
```

不实现网络同步。

## 阶段 8：文档和清理

完成：

- ADR
- 集成指南
- 错误码
- 版本兼容
- 后续 EPIC-03/05 接口清单

---

# 32. 推荐提交拆分

```text
docs(contracts): audit current domain and api models
feat(contracts): add shared contract crate
feat(contracts): define common ids money and timestamps
feat(contracts): define finance and habit entities
feat(contracts): define notes english and workout entities
feat(contracts): add entity type registry
feat(sync): define sync protocol v1
test(sync): add in-memory protocol testkit
feat(contracts): generate json schema openapi and typescript
feat(desktop): add contract adapters
docs(sync): document protocol v1 and compatibility policy
```

不得把整个 EPIC 放在一个无法审查的超大提交中。

---

# 33. Definition of Done

只有全部满足才可认定 EPIC-02 完成：

- [ ] 已生成当前契约审计
- [ ] 有独立 `lifetrace-contracts` crate
- [ ] contract crate 不依赖 Tauri、Axum、rusqlite
- [ ] 财务、习惯、复盘、笔记、英语、训记领域 DTO 已定义
- [ ] 文件元数据与跨实体关联 DTO 已定义
- [ ] 用户和设备 DTO 已定义
- [ ] 所有 entity type 有稳定注册表
- [ ] 同步协议 v1 已定义
- [ ] Push 幂等语义明确
- [ ] Pull 使用服务端 cursor
- [ ] Delete 使用 tombstone
- [ ] Snapshot 协议已定义
- [ ] 冲突不依赖客户端时间
- [ ] `version` 与 `serverVersion` 职责分离
- [ ] protocol/schema/client/app 版本职责明确
- [ ] 未知字段可兼容
- [ ] 未知枚举不会导致整批同步崩溃
- [ ] 有稳定错误码
- [ ] 有 Golden Fixture
- [ ] 有 JSON Schema
- [ ] 有 TypeScript 生成物
- [ ] 有 OpenAPI
- [ ] 有内存参考实现
- [ ] 重复 changeId 测试通过
- [ ] changeId 不同 payload 重用测试通过
- [ ] 冲突测试通过
- [ ] tombstone 测试通过
- [ ] cursor 分页测试通过
- [ ] snapshot 测试通过
- [ ] atomic group 测试通过
- [ ] 桌面端至少一个实体完成契约适配
- [ ] 未提前实现生产同步 Worker
- [ ] 前端测试通过
- [ ] Rust 测试通过
- [ ] 构建通过
- [ ] 文档完整

---

# 34. Agent 完整执行提示词

下面内容可以直接复制给 Codex、Claude Code 或其他编码 Agent。

```text
你现在负责完成 LifeTrace 仓库的 EPIC-02：统一领域模型与同步协议。

仓库：
zhouxingxing1279/LifeTrace

当前技术栈：
Tauri 2 + React/Vite + Rust/Axum + rusqlite + SQLite。

前置条件：
EPIC-01 数据层重构的目标结构已经确定。执行时必须先检查仓库当前状态，不能假设 EPIC-01 已经完全实现。

本 Epic 的目标是建立公共领域契约和同步协议 v1，不是实现生产云端服务或客户端后台同步。

一、开始前必须阅读

- README.md
- package.json
- src/types/index.ts
- src/types/english.ts
- src-tauri/Cargo.toml
- src-tauri/src/lib.rs
- src-tauri/src/server.rs
- src-tauri/src/server/state.rs
- src-tauri/src/server/migration.rs
- src-tauri/src/server/notes.rs
- src-tauri/src/server/english.rs
- src-tauri/src/server/imports.rs
- src-tauri/src/server/xunji.rs
- src-tauri/src/database/**
- docs/epic-01/**

搜索整个仓库中的：

- interface
- type
- struct
- enum
- serde
- data_json
- amount
- amount_cents
- createdAt
- updatedAt
- version
- device
- sync
- outbox
- cursor
- change_id
- schema_version
- backup

二、本轮第一步只能执行只读审计

先生成：

docs/epic-02/current-contract-audit.md

审计必须列出：

1. 当前所有 TypeScript 领域类型
2. 当前所有 Rust 领域表示
3. EPIC-01 数据表和字段
4. API DTO
5. 备份 DTO
6. camelCase 和 snake_case 差异
7. 金额表示差异
8. 时间表示差异
9. ID 类型差异
10. nullable 差异
11. 枚举差异
12. 哪些实体需要同步
13. 哪些实体由服务端管理
14. 哪些实体只在设备本地
15. 哪些凭据永远不得同步
16. 对 EPIC-02 方案必须调整的地方

审计完成前不要批量修改领域类型。

三、硬性边界

本 Epic 必须完成：

- 公共 Rust contract crate
- 公共领域 DTO
- entity type registry
- Push/Pull/Snapshot/Tombstone/Conflict DTO
- 版本兼容
- 错误码
- JSON Schema
- TypeScript 生成
- OpenAPI
- Golden Fixture
- 内存同步参考实现
- 桌面契约适配示例

本 Epic不得实现：

- PostgreSQL 云端服务
- 正式认证
- 正式设备注册
- 正式 sync outbox
- 后台网络 Worker
- 自动重试调度
- 同步 UI
- 对象存储
- Android App

这些属于 EPIC-03、EPIC-04、EPIC-05 和 EPIC-12。

四、公共契约 crate

新增：

crates/lifetrace-contracts

要求：

- 不依赖 Tauri
- 不依赖 Axum
- 不依赖 rusqlite
- 不依赖 React
- 使用 serde
- 使用 schemars 生成 JSON Schema
- 使用 ts-rs 或等效稳定方式生成 TypeScript
- Rust 是公共领域和 Wire DTO 的权威来源
- 数据库 Row、Domain Model、Wire DTO 和 UI View Model 分离

五、基础值对象

必须定义：

- UserId
- DeviceId
- EntityId
- ChangeId
- RequestId
- ConflictId
- Cursor
- ServerVersion
- MoneyAmount
- CurrencyCode
- EntityMeta

所有 ID 在线路 JSON 中使用字符串。

金额在线路中必须使用 amountCents 整数，不使用 amount 浮点数。

时间点使用 RFC3339 UTC。
自然日使用 YYYY-MM-DD。

Cursor、serverVersion 和 baseServerVersion 在线路 JSON 中使用字符串，避免 JavaScript 安全整数问题。

六、本地 version 与 serverVersion 分离

不得直接使用 EPIC-01 的本地 version 作为服务器权威版本。

统一：

- localVersion：本地修订号
- serverVersion：服务器权威实体版本
- baseServerVersion：客户端提交时已知服务器版本

离线修改不得伪造新的 serverVersion。

七、领域 DTO

按当前实际业务定义：

- identity.user
- identity.device

- finance.account
- finance.category
- finance.transaction
- finance.transaction_evidence

- habit.activity
- habit.log
- review.daily

- note.folder
- note.note
- note.tag
- note.tag_relation
- note.relation
- note.revision

- english.article
- english.learning_record
- english.highlight
- english.note
- english.vocabulary
- english.vocabulary_occurrence
- english.vocabulary_review_state

- workout.import
- workout.workout
- workout.exercise
- workout.set
- workout.training_note

- file.metadata
- entity.link
- user.preference

每个 entity type 必须登记：

- schemaVersion
- ownership
- syncMode
- conflictMode
- containsFileReferences

必须明确以下类型：

- user_owned
- server_managed
- shared_catalog
- device_local
- secret_local_only

API Key、密码、邮箱授权码、Refresh Token、私钥和证书私钥必须为 secret_local_only，不能进入普通 Sync Payload。

八、Sync Change v1

Change 必须包含：

- changeId
- entityType
- entityId
- operation
- baseServerVersion
- entitySchemaVersion
- clientModifiedAt
- payload
- atomicGroupId
- dependencies

operation 仅：

- upsert
- delete

upsert payload 是完整实体快照，不使用 JSON Patch。

delete 默认不携带实体 payload，服务端必须生成 tombstone。

clientModifiedAt 只用于审计，不能用于全局顺序或冲突自动解决。

九、Push

定义：

POST /api/v1/sync/push

PushRequest：

- requestId
- client
- changes

PushResponse 每个 Change 返回：

- accepted
- duplicate
- conflict
- rejected

幂等键：

userId + changeId

相同 changeId 和相同 payload：
返回首次结果，不重复写入。

相同 changeId 和不同 payload：
返回 LIFETRACE_CHANGE_ID_REUSE。

requestId 只用于追踪，不替代 changeId。

十、Atomic Group

相同 atomicGroupId 的 Changes：

- 必须在同一请求中
- 全部成功或全部失败
- 任意一项冲突则整个 group 失败
- group 最大数量由 capabilities 声明
- 内存参考实现必须验证

十一、Pull

定义：

POST /api/v1/sync/pull

Pull 使用：

- afterCursor
- limit
- entityTypes

响应 Change 必须按服务端 cursor 严格升序。

客户端不得按 updatedAt 重新排序。

Cursor 是 opaque string，客户端不能加减或猜测。

十二、Tombstone

Delete 必须产生：

- entityType
- entityId
- deletedAt
- serverVersion
- deletedByDevice

被删除实体重新创建必须使用新 ID。
恢复软删除必须显式基于 tombstone 的最新 serverVersion 提交。

十三、Snapshot

定义：

POST /api/v1/sync/snapshot

必须支持：

- snapshotId
- snapshotCursor
- pageToken
- entityTypes
- pageSize
- items
- nextPageToken
- completed

同一 snapshot 的所有页面必须对应一致视图。

客户端完成 snapshot 后：

- 将 cursor 设置为 snapshotCursor
- 再从 snapshotCursor 开始 Pull

必须通过测试证明 Snapshot 与随后 Pull 不漏并发变化。

十四、Capabilities

定义：

GET /api/v1/sync/capabilities

至少返回：

- protocolVersion
- supportedProtocolVersions
- schemaVersion
- minimumSchemaVersion
- minimumClientVersions
- maximumPushBatchSize
- maximumPullBatchSize
- maximumRequestBytes
- maximumSnapshotPageSize
- tombstoneRetentionDays
- supportedEntityTypes
- serverTime

十五、版本规则

必须区分：

- protocolVersion
- schemaVersion
- entitySchemaVersion
- clientVersion
- appId

v1 允许新增可选字段和新 entity type。
破坏性字段修改必须升级 protocolVersion。

未知 JSON 字段必须可忽略。

未知枚举不能导致整个同步批次解析失败，使用 Unknown(String) 或字符串 newtype 等向前兼容方案。

客户端太旧返回：

HTTP 426
LIFETRACE_CLIENT_TOO_OLD

十六、冲突

禁止默认 Last Write Wins。

当：

client baseServerVersion != current serverVersion

返回 conflict，并包含服务器当前实体或 tombstone。

解决方式由后续客户端实现：

- keep_server
- keep_local
- manual_merge

keep_local 必须生成新的 changeId，并基于服务器最新版本重新提交。

十七、错误码

实现稳定 ErrorCode，至少包括：

- LIFETRACE_PROTOCOL_UNSUPPORTED
- LIFETRACE_SCHEMA_UNSUPPORTED
- LIFETRACE_CLIENT_TOO_OLD
- LIFETRACE_DEVICE_NOT_REGISTERED
- LIFETRACE_INVALID_REQUEST
- LIFETRACE_BATCH_TOO_LARGE
- LIFETRACE_PAYLOAD_TOO_LARGE
- LIFETRACE_UNKNOWN_ENTITY_TYPE
- LIFETRACE_INVALID_ENTITY_PAYLOAD
- LIFETRACE_DEPENDENCY_MISSING
- LIFETRACE_CHANGE_ID_REUSE
- LIFETRACE_BASE_VERSION_MISMATCH
- LIFETRACE_CURSOR_INVALID
- LIFETRACE_CURSOR_EXPIRED
- LIFETRACE_SNAPSHOT_REQUIRED
- LIFETRACE_ATOMIC_GROUP_FAILED
- LIFETRACE_INTERNAL_ERROR

错误码发布后不得改变含义。

十八、内存参考实现

实现测试专用同步服务器状态机，不作为生产服务。

必须测试：

1. Create
2. Update
3. Base version conflict
4. 重复 changeId
5. changeId 不同 payload 重用
6. Delete tombstone
7. 双方删除
8. Pull cursor 顺序
9. Pull 分页
10. Snapshot 一致性
11. Snapshot 后 Pull
12. Atomic group
13. 未知 entity type
14. 过期 cursor

十九、生成物

新增 contract exporter，生成：

- contracts/json-schema/*.schema.json
- contracts/openapi/lifetrace-sync-v1.json
- contracts/typescript/lifetrace-contracts.generated.ts

生成结果必须稳定。
重复生成不得产生无意义 diff。
generated 文件必须标注禁止手工编辑。

增加脚本：

- contracts:generate
- contracts:test
- contracts:check

二十、桌面端适配

本 Epic 只做契约适配，不做网络同步。

至少完成一条财务交易：

SQLite/Domain Transaction
→ Contract Transaction
→ SyncChangeV1
→ JSON
→ 反序列化
→ 字段一致性测试

不要建立生产 sync outbox、后台 Worker 或 HTTP 调用。

二十一、文档

必须生成：

- docs/epic-02/current-contract-audit.md
- docs/epic-02/domain-model.md
- docs/epic-02/entity-registry.md
- docs/epic-02/sync-protocol-v1.md
- docs/epic-02/compatibility-policy.md
- docs/epic-02/error-codes.md
- docs/epic-02/integration-guide.md

以及 ADR：

- contract source
- full snapshot changes
- server cursor
- conflict policy
- versioning

二十二、测试与检查

运行：

cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml
npm.cmd run lint
npm.cmd test
npm.cmd run test:rust
npm.cmd run build

生成物检查：

npm.cmd run contracts:check

二十三、分阶段执行和提交

阶段 0：契约审计
阶段 1：契约 crate 基础
阶段 2：领域 DTO
阶段 3：Entity Registry
阶段 4：Sync v1 DTO
阶段 5：参考实现与语义测试
阶段 6：Schema/OpenAPI/TS 生成
阶段 7：桌面适配示例
阶段 8：文档和清理

每个阶段单独提交，不要一次性完成全部修改后再测试。

二十四、最终报告

完成后输出：

1. 实际修改文件
2. 公共实体清单
3. Entity Registry
4. 同步协议 DTO 清单
5. 协议版本规则
6. 错误码清单
7. Golden Fixture 清单
8. JSON Schema 和 OpenAPI 输出
9. 参考实现测试结果
10. 全仓库测试结果
11. 未解决问题
12. EPIC-03 云端服务需要实现的接口
13. EPIC-05 客户端同步需要实现的接口

在所有 Definition of Done 满足前，不要声称 EPIC-02 已完成。
```

---

# 35. 推荐的实际使用方式

不要只把上面的完整提示词一次性丢给 Agent，让它连续修改整个仓库。

建议：

1. 将本文件放入仓库：

```text
docs/epic-02/implementation-plan.md
```

2. 第一次只让 Agent 执行“阶段 0：契约审计”
3. 审查审计文档
4. 再按阶段 1～8 分别执行
5. 每个阶段完成后检查测试和生成物
6. EPIC-02 完成后再进入 EPIC-03 和 EPIC-05

第一轮执行提示词：

```text
请阅读 docs/epic-02/implementation-plan.md。

本轮只完成“阶段 0：契约审计”，不得创建同步服务器、不得创建后台同步 Worker、不得批量修改现有领域类型。

生成 docs/epic-02/current-contract-audit.md，并输出：

- 阅读过的文件
- 当前 TypeScript 和 Rust 领域模型
- EPIC-01 数据模型状态
- 字段命名差异
- ID、金额、时间、版本差异
- 需要同步与禁止同步的实体
- 建议的公共 Contract 模块拆分
- 实施方案中需要根据实际代码调整的内容
- 下一阶段预计修改文件

完成后停止。
```
