# LifeTrace EPIC-03：独立云端后端服务——Agent 实施方案

> 目标仓库：`zhouxingxing1279/LifeTrace`  
> 前置依赖：EPIC-01 已完成；EPIC-02 公共契约与同步协议必须达到可编译、可测试状态。  
> 目标技术栈：Rust + Axum + PostgreSQL + SQLx + Docker。  
> 核心原则：服务独立、契约复用、用户隔离、服务端排序、幂等提交、显式冲突、删除墓碑、可迁移、可测试。

---

## 1. EPIC-03 完成后的系统形态

```text
LifeTrace Desktop / Android / Web
                ↓ HTTPS
         LifeTrace Cloud
          Rust + Axum
                ↓
           PostgreSQL
```

云端服务需要提供：

- 配置加载
- PostgreSQL 连接池与 Migration
- 健康检查和就绪检查
- 同步 Capabilities
- Push
- Pull
- Snapshot
- 服务端 Cursor
- 服务端 Entity Version
- Change ID 幂等
- 冲突响应
- Tombstone
- Atomic Group
- 用户数据隔离
- 结构化日志
- 优雅关闭
- Docker 本地环境与部署示例
- 集成测试

EPIC-03 的最终产物不是一个嵌入 Tauri 的本地接口，而是一个可以独立编译、独立运行和独立部署的云端服务。

---

## 2. 开始实施前必须检查

Agent 必须先阅读：

```text
README.md
package.json
Cargo.toml（如果 EPIC-02 已建立 Workspace）

src-tauri/Cargo.toml
src-tauri/src/server.rs
src-tauri/src/database/**

docs/epic-01/**
docs/epic-02/**

crates/lifetrace-contracts/**
contracts/openapi/**
contracts/json-schema/**
```

必须确认：

1. EPIC-01 的真实列数据模型已经完成。
2. 金额在线路中使用整数分。
3. `localVersion` 和 `serverVersion` 已分离。
4. EPIC-02 已定义 `EntityType`、`SyncChangeV1`、Push、Pull、Snapshot、Tombstone、Conflict 和 ErrorCode。
5. Entity Registry 已明确哪些实体可同步。
6. 每个同步实体都有稳定单一 ID。
7. 关系实体没有只依赖复合主键而缺少同步 ID。
8. 未知字段和新增枚举具备向前兼容能力。

若 EPIC-02 尚未完成，先生成：

```text
docs/epic-03/precondition-report.md
```

说明阻塞项，然后停止。不得在云端项目中重新定义第二套同步协议。

---

## 3. 与其他 Epic 的边界

### EPIC-03 必须实现

- 独立云端服务
- PostgreSQL
- SQLx Migration
- 通用实体存储
- Push/Pull/Snapshot
- Change Log
- Cursor
- Server Version
- 幂等
- Conflict
- Tombstone
- Atomic Group
- 开发认证接口
- 用户隔离
- Docker/Compose
- 基础 Caddy 示例
- 测试和文档

### EPIC-04 实现

EPIC-03 不实现正式登录系统。EPIC-04 负责：

- 注册和登录
- 密码哈希
- Access Token
- Refresh Token
- Token 轮换与撤销
- 忘记密码
- 邮箱验证
- 设备注册与撤销
- 登录限流

EPIC-03 只定义并使用：

```text
AuthProvider
AuthenticatedPrincipal
UserId
DeviceId
DevelopmentAuthProvider
TestAuthProvider
```

### EPIC-05 实现

- SQLite Sync Outbox
- 客户端 Push Worker
- 客户端 Pull Worker
- 本地 Cursor
- 网络重试
- 冲突 UI
- 同步状态
- 后台同步

### EPIC-12 实现

- S3/MinIO
- 文件二进制上传
- 分片上传
- 预签名 URL
- 文件下载

EPIC-03 只同步 `file.metadata`，不处理文件二进制。

---

## 4. 核心架构决策

### 4.1 云端服务必须独立

禁止继续把云端功能写入：

```text
src-tauri/src/server.rs
```

新增：

```text
services/lifetrace-cloud/
```

云端服务不得依赖：

- Tauri
- rusqlite
- React
- 桌面本地服务

### 4.2 直接复用 EPIC-02 Contract

云端必须依赖：

```text
crates/lifetrace-contracts
```

不得重复定义：

- Entity DTO
- Sync DTO
- Cursor
- ServerVersion
- ErrorCode
- Entity Registry

数据库内部 Model 可以独立，但所有 HTTP Wire Contract 必须来自公共 Contract crate。

### 4.3 PostgreSQL 使用通用同步实体存储

EPIC-03 不立即把 EPIC-01 的所有业务表逐张复制到 PostgreSQL。

云端同步权威副本使用：

```text
sync_entities
```

保存经过 Contract 校验的完整 Entity Payload。

原因：

- 同步引擎可以统一处理所有实体。
- 新 Entity Type 不需要重新编写整套云端 Repository。
- 云端仍保存完整 PostgreSQL 数据副本。
- 后续 Web 和统计能力可以建立领域投影。
- 避免本 Epic 同时重复实现几十张业务表。

注意：

- JSONB 不是无约束 JSON。
- 写入前必须反序列化为 EPIC-02 的具体实体类型。
- 未注册 Entity Type 必须拒绝。
- `secret_local_only` 类型永远不能进入同步表。

---

## 5. 推荐目录结构

```text
services/
└── lifetrace-cloud/
    ├── Cargo.toml
    ├── Dockerfile
    ├── .dockerignore
    ├── migrations/
    │   ├── 0001_cloud_identity.sql
    │   ├── 0002_sync_entities.sql
    │   ├── 0003_processed_changes.sql
    │   ├── 0004_change_log.sql
    │   ├── 0005_snapshots.sql
    │   └── 0006_indexes.sql
    ├── src/
    │   ├── main.rs
    │   ├── lib.rs
    │   ├── app.rs
    │   ├── config.rs
    │   ├── state.rs
    │   ├── error.rs
    │   ├── shutdown.rs
    │   ├── api/
    │   │   ├── health.rs
    │   │   └── v1/sync/
    │   │       ├── capabilities.rs
    │   │       ├── push.rs
    │   │       ├── pull.rs
    │   │       └── snapshot.rs
    │   ├── auth/
    │   │   ├── provider.rs
    │   │   ├── principal.rs
    │   │   ├── development.rs
    │   │   └── testing.rs
    │   ├── db/
    │   │   ├── pool.rs
    │   │   ├── migrate.rs
    │   │   └── models.rs
    │   ├── repository/
    │   │   ├── entities.rs
    │   │   ├── changes.rs
    │   │   ├── processed_changes.rs
    │   │   └── snapshots.rs
    │   ├── sync/
    │   │   ├── validator.rs
    │   │   ├── canonical_json.rs
    │   │   ├── payload_hash.rs
    │   │   ├── cursor_codec.rs
    │   │   ├── page_token.rs
    │   │   ├── push_service.rs
    │   │   ├── pull_service.rs
    │   │   ├── snapshot_service.rs
    │   │   ├── dependencies.rs
    │   │   └── maintenance.rs
    │   └── telemetry/
    │       ├── logging.rs
    │       └── request_id.rs
    └── tests/
        ├── health_test.rs
        ├── push_test.rs
        ├── pull_test.rs
        ├── conflict_test.rs
        ├── idempotency_test.rs
        ├── snapshot_test.rs
        ├── atomic_group_test.rs
        └── isolation_test.rs

deploy/cloud/
├── docker-compose.local.yml
├── docker-compose.test.yml
├── docker-compose.production.example.yml
├── Caddyfile.example
└── .env.example

scripts/cloud/
├── dev-up.ps1
├── dev-down.ps1
├── migrate.ps1
├── test.ps1
└── smoke-test.ps1

docs/epic-03/
├── current-cloud-audit.md
├── architecture.md
├── database-schema.md
├── api-implementation.md
├── authentication-boundary.md
├── local-development.md
├── deployment.md
├── security-review.md
└── completion-report.md
```

---

## 6. 依赖要求

建议使用：

```text
axum
tokio
sqlx
serde
serde_json
uuid
chrono
thiserror
tower
tower-http
tracing
tracing-subscriber
sha2
hmac
base64
```

SQLx 启用：

```text
runtime-tokio-rustls
postgres
uuid
chrono
json
migrate
```

要求：

- 不要求全局安装 `sqlx-cli`。
- 编译不能依赖实时 PostgreSQL Schema。
- 优先使用运行时 `query/query_as`，或提交稳定的 SQLx 离线元数据。
- 所有版本必须固定在 Cargo.lock，不使用不受控版本。
- 云端服务不得引入 rusqlite。

---

## 7. 配置设计

定义强类型 `CloudConfig`，至少包含：

```text
environment
bindAddress
databaseUrl
databaseMinConnections
databaseMaxConnections
migrationOnStartup

requestBodyLimitBytes
pushMaxChanges
pullMaxChanges
snapshotMaxPageSize
maximumAtomicGroupSize

cursorSigningKey
pageTokenSigningKey

corsAllowedOrigins

developmentAuthEnabled
developmentAuthUserId
developmentAuthDeviceId
developmentAuthToken

snapshotTtlSeconds
maintenanceIntervalSeconds
gracefulShutdownSeconds
```

环境变量建议：

```text
LIFETRACE_ENV
LIFETRACE_BIND_ADDRESS
DATABASE_URL
DATABASE_MIN_CONNECTIONS
DATABASE_MAX_CONNECTIONS
MIGRATION_ON_STARTUP

REQUEST_BODY_LIMIT_BYTES
PUSH_MAX_CHANGES
PULL_MAX_CHANGES
SNAPSHOT_MAX_PAGE_SIZE
MAXIMUM_ATOMIC_GROUP_SIZE

CURSOR_SIGNING_KEY
PAGE_TOKEN_SIGNING_KEY
CORS_ALLOWED_ORIGINS

DEV_AUTH_ENABLED
DEV_AUTH_USER_ID
DEV_AUTH_DEVICE_ID
DEV_AUTH_TOKEN

SNAPSHOT_TTL_SECONDS
MAINTENANCE_INTERVAL_SECONDS
GRACEFUL_SHUTDOWN_SECONDS

RUST_LOG
```

硬性要求：

- 缺少数据库地址时启动失败。
- 生产环境缺少签名密钥时启动失败。
- 生产环境启用 DEV_AUTH 时启动失败。
- CORS 不得默认 `*`。
- 密钥不能出现在 Debug 或日志中。
- `.env` 不提交 Git。
- 错误信息不得暴露完整 DATABASE_URL。

---

## 8. 启动流程

```text
加载配置
→ 初始化 tracing
→ 创建 PostgreSQL Pool
→ 验证数据库
→ 执行 Migration
→ 初始化 AuthProvider
→ 初始化 Cursor/Page Token Codec
→ 构建 AppState
→ 启动 Maintenance Worker
→ 构建 Router
→ 监听端口
→ 优雅关闭
```

禁止：

- Migration 失败后继续 Ready。
- 数据库不可用时 Ready 返回 200。
- DEV_AUTH 配置错误时静默回退。
- 在生产环境自动创建测试用户。

---

## 9. PostgreSQL Schema

### 9.1 cloud_users

EPIC-03 只创建身份锚点，不存登录凭据：

```sql
CREATE TABLE cloud_users (
    id UUID PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

不得包含：

```text
email
password_hash
refresh_token
verification_token
```

### 9.2 cloud_devices

```sql
CREATE TABLE cloud_devices (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    app_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    client_version TEXT,
    protocol_version INTEGER,
    schema_version INTEGER,

    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'revoked')),

    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

正式注册、撤销和设备管理由 EPIC-04 实现。

### 9.3 sync_entities

```sql
CREATE TABLE sync_entities (
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_schema_version INTEGER NOT NULL,

    server_version BIGINT NOT NULL
        CHECK (server_version > 0),

    payload JSONB,
    payload_hash BYTEA,

    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMPTZ,

    origin_device_id UUID,
    created_at TIMESTAMPTZ NOT NULL,
    server_modified_at TIMESTAMPTZ NOT NULL,
    client_modified_at TIMESTAMPTZ,

    last_cursor BIGINT NOT NULL,

    PRIMARY KEY (user_id, entity_type, entity_id),

    CHECK (
        (
            is_deleted = FALSE
            AND payload IS NOT NULL
            AND payload_hash IS NOT NULL
            AND deleted_at IS NULL
        )
        OR
        (
            is_deleted = TRUE
            AND payload IS NULL
            AND deleted_at IS NOT NULL
        )
    )
);
```

索引：

```sql
CREATE INDEX idx_sync_entities_user_type
ON sync_entities(user_id, entity_type);

CREATE INDEX idx_sync_entities_user_active
ON sync_entities(user_id, is_deleted, entity_type);

CREATE INDEX idx_sync_entities_last_cursor
ON sync_entities(user_id, last_cursor);
```

`entity_id` 默认使用 TEXT，因为 EPIC-01 保留历史 ID。只有 EPIC-02 已证明全部 ID 都是 UUID 时才改为 UUID。

### 9.4 sync_change_log

```sql
CREATE TABLE sync_change_log (
    cursor BIGSERIAL PRIMARY KEY,

    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,

    operation TEXT NOT NULL
        CHECK (operation IN ('upsert', 'delete')),

    entity_schema_version INTEGER NOT NULL,
    server_version BIGINT NOT NULL,

    payload JSONB,
    payload_hash BYTEA,
    tombstone JSONB,

    origin_device_id UUID,
    client_modified_at TIMESTAMPTZ,
    server_modified_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

索引：

```sql
CREATE INDEX idx_change_log_user_cursor
ON sync_change_log(user_id, cursor);

CREATE INDEX idx_change_log_user_type_cursor
ON sync_change_log(user_id, entity_type, cursor);
```

### 9.5 sync_processed_changes

```sql
CREATE TABLE sync_processed_changes (
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    change_id UUID NOT NULL,
    request_hash BYTEA NOT NULL,

    result_status TEXT NOT NULL,
    result_json JSONB NOT NULL,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,

    processed_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, change_id)
);
```

### 9.6 Snapshot

```sql
CREATE TABLE sync_snapshots (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL
        REFERENCES cloud_users(id)
        ON DELETE CASCADE,

    scope_hash BYTEA NOT NULL,
    snapshot_cursor BIGINT NOT NULL,

    status TEXT NOT NULL
        CHECK (status IN ('building', 'ready', 'failed', 'expired')),

    item_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    error_message TEXT
);

CREATE TABLE sync_snapshot_items (
    snapshot_id UUID NOT NULL
        REFERENCES sync_snapshots(id)
        ON DELETE CASCADE,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_schema_version INTEGER NOT NULL,
    server_version BIGINT NOT NULL,
    payload JSONB NOT NULL,
    payload_hash BYTEA NOT NULL,
    server_modified_at TIMESTAMPTZ NOT NULL,

    PRIMARY KEY (snapshot_id, entity_type, entity_id)
);
```

---

## 10. Canonical JSON 与 Hash

相同语义的 JSON 必须生成相同 Hash：

```json
{"a":1,"b":2}
```

和：

```json
{"b":2,"a":1}
```

不得产生不同 Hash。

实现要求：

- 使用稳定 JSON Canonicalization。
- 至少递归排序 Object Key。
- 稳定处理数字、字符串、数组和 Null。
- 不直接 Hash 原始 HTTP Body。
- 不依赖 HashMap 插入顺序。

Change Hash 至少包含：

```text
entityType
entityId
operation
baseServerVersion
entitySchemaVersion
payload
atomicGroupId
dependencies
```

不包含：

```text
requestId
服务端时间
Header 顺序
```

---

## 11. 认证边界

定义：

```rust
pub trait AuthProvider {
    async fn authenticate(...) -> Result<AuthenticatedPrincipal, ApiErrorV1>;
}
```

Principal：

```text
userId
deviceId
appId
```

### DevelopmentAuthProvider

使用固定 Bearer Token：

```text
Authorization: Bearer <DEV_AUTH_TOKEN>
```

映射到固定测试用户和设备。

禁止：

```text
X-User-Id: 任意用户
```

硬性规则：

- `LIFETRACE_ENV=production` 时 DEV_AUTH 必须关闭。
- 生产环境启用 DEV_AUTH 时拒绝启动。
- DEV Token 不记录日志。
- 测试使用 TestAuthProvider。
- EPIC-04 替换 AuthProvider 时不修改同步业务层。

无需认证的接口：

```text
GET /health/live
GET /health/ready
GET /api/v1/sync/capabilities
```

数据同步接口必须认证。

---

## 12. API

```text
GET  /health/live
GET  /health/ready
GET  /api/v1/meta/version

GET  /api/v1/sync/capabilities
POST /api/v1/sync/push
POST /api/v1/sync/pull
POST /api/v1/sync/snapshot
```

本 Epic 不实现：

```text
/auth/*
/users/register
/devices/register
/files/upload
/mail/*
/ai/*
```

---

## 13. Health

### Liveness

```text
GET /health/live
```

只检查进程存活，不查询数据库。

### Readiness

```text
GET /health/ready
```

检查：

- PostgreSQL 可连接
- Migration 已完成
- 配置有效
- AuthProvider 已初始化

数据库不可用时返回 503。

---

## 14. Payload 校验

写入前执行：

```text
Entity Type
→ Entity Registry
→ Entity Schema Version
→ Typed DTO Deserialize
→ Domain Invariant Validate
→ Canonical Serialize
→ Hash
```

必须校验：

- Entity Type 已注册。
- Schema Version 受支持。
- Payload Entity ID 与 Envelope 一致。
- Payload User ID 与 Auth 用户一致，或由服务端覆盖。
- 不包含 `secret_local_only` 数据。
- Upsert 必须有完整 Payload。
- Delete 不得携带 Upsert Payload。
- 客户端不能设置 Server Version。
- 客户端不能设置 Server Modified Time。

禁止只检查 JSON 是 Object 就直接入库。

---

## 15. Push

处理流程：

```text
认证
→ 协议版本校验
→ 批量大小校验
→ Payload 校验
→ Canonical Hash
→ Atomic Group 分组
→ 每组数据库事务
→ 返回结果
```

### 15.1 Create

条件：

```text
实体不存在
baseServerVersion = 0
operation = upsert
```

执行：

1. `serverVersion = 1`
2. 写 Change Log 并取得 Cursor
3. 写 sync_entities
4. 写 processed_changes
5. 返回 accepted

### 15.2 Update

条件：

```text
currentServerVersion = baseServerVersion
```

执行：

1. `SELECT ... FOR UPDATE`
2. `serverVersion + 1`
3. 写 Change Log
4. 更新 sync_entities
5. 写 processed_changes

### 15.3 Conflict

条件：

```text
currentServerVersion != baseServerVersion
```

返回：

- 当前 Server Version
- 当前服务器 Payload
- 或 Tombstone
- Conflict Reason

不得：

- 使用客户端时间判断胜负
- 默认 Last Write Wins
- 修改服务端实体

### 15.4 Change ID 幂等

幂等键：

```text
userId + changeId
```

相同 Change ID、相同 Hash：

```text
duplicate
```

必须返回第一次结果，不重复执行。

相同 Change ID、不同 Hash：

```text
LIFETRACE_CHANGE_ID_REUSE
```

### 15.5 固定锁顺序

Atomic Group 内按：

```text
entityType + entityId
```

排序后获取锁，降低死锁风险。

---

## 16. Delete 与 Tombstone

Delete 必须：

1. 校验 Base Version。
2. `serverVersion + 1`。
3. 生成 Tombstone。
4. 写 Change Log。
5. 更新 `sync_entities`：
   - `payload=NULL`
   - `payload_hash=NULL`
   - `is_deleted=true`
   - `deleted_at=server time`
6. 写 processed_changes。

不得硬删除实体或变更日志。

---

## 17. Atomic Group

同一 `atomicGroupId` 的 Changes：

```text
BEGIN
→ 校验全部 Changes
→ 校验幂等
→ 按固定顺序锁定实体
→ 校验 Base Version
→ 校验依赖
→ 写全部实体
→ 写全部 Change Log
→ 写全部 processed_changes
→ COMMIT
```

任意一项失败：

```text
ROLLBACK
```

整个 Group 返回失败。

依赖处理：

- 同组新建实体可以满足依赖。
- 使用拓扑排序。
- 循环依赖返回 INVALID_REQUEST。
- 跨组依赖必须已经存在于服务端。

---

## 18. Cursor

PostgreSQL 内部使用：

```text
BIGSERIAL / BIGINT
```

线路 Cursor 是签名 opaque token，包含：

```text
protocolVersion
userId
scopeHash
cursorPosition
issuedAt
```

使用 HMAC-SHA256。

要求：

- Cursor 绑定用户。
- Cursor 绑定 Entity Scope。
- Cursor 被篡改时拒绝。
- 不同用户不能互用 Cursor。
- Scope 改变时要求 Snapshot。
- 客户端不能对 Cursor 做加减。

---

## 19. Pull

Pull 必须：

- 按 `cursor` 严格升序。
- 所有查询带 `user_id`。
- 支持 Entity Scope。
- 支持 Limit。
- 返回 Tombstone。
- 正确计算 `nextCursor`。
- 正确计算 `hasMore`。
- 分页无重复、无遗漏。

核心查询：

```sql
SELECT ...
FROM sync_change_log
WHERE user_id = $1
  AND cursor > $2
  AND entity_type = ANY($3)
ORDER BY cursor ASC
LIMIT $4;
```

不得根据 `updatedAt` 重新排序。

---

## 20. Snapshot

### 20.1 创建

使用：

```text
REPEATABLE READ
```

流程：

1. 获取当前用户最大 Cursor。
2. 创建 Snapshot。
3. 将 Scope 内、未删除的 Entity 复制到 `sync_snapshot_items`。
4. 标记 ready。
5. 提交事务。
6. 返回第一页。

### 20.2 分页

使用 Keyset Pagination：

```sql
WHERE snapshot_id = $1
  AND (entity_type, entity_id) > ($2, $3)
ORDER BY entity_type, entity_id
LIMIT $4;
```

禁止 OFFSET Pagination。

### 20.3 Page Token

Page Token 绑定：

```text
userId
snapshotId
scopeHash
lastEntityType
lastEntityId
expiresAt
```

使用独立 HMAC 签名。

### 20.4 完成

最后一页返回：

```text
completed=true
snapshotCursor
nextPageToken=null
```

客户端之后从 `snapshotCursor` Pull。

### 20.5 清理

Maintenance Worker 定期清理过期 Snapshot 和 Snapshot Items。

---

## 21. 用户隔离

所有 Repository 查询必须显式包含：

```text
user_id
```

禁止仅凭：

```text
entity_id
change_id
snapshot_id
```

查数据。

必须测试：

- 用户 A 无法读取用户 B 数据。
- 相同 Entity ID 在不同用户下互不影响。
- 相同 Change ID 在不同用户下互不影响。
- 用户 A 不能使用用户 B 的 Cursor。
- 用户 A 不能使用用户 B 的 Snapshot Token。
- 错误响应不泄露目标 Entity 是否存在。

RLS 可以作为后续防御层，但不能替代应用层 `user_id` 过滤。

---

## 22. Middleware

至少实现：

```text
Request ID
Structured Tracing
Body Size Limit
Request Timeout
CORS
Sensitive Header Redaction
Auth Extractor
```

要求：

- 响应包含 Request ID。
- CORS 从配置读取白名单。
- 生产不允许 Any Origin。
- Timeout 触发时数据库事务必须回滚。
- Authorization Header 不进入日志。

---

## 23. 日志

可以记录：

```text
requestId
route
status
latency
deviceId
changeCount
acceptedCount
conflictCount
rejectedCount
cursorRange
snapshotItemCount
```

不得记录：

```text
Bearer Token
DEV_AUTH_TOKEN
DATABASE_URL
完整 Entity Payload
笔记正文
API Key
密码
Refresh Token
```

---

## 24. Docker 与本地开发

### docker-compose.local.yml

至少包含：

```text
postgres
lifetrace-cloud（可选 profile）
```

要求：

- PostgreSQL 固定 Major Version，不使用 `latest`。
- 使用持久化 Volume。
- 配置 Healthcheck。
- 账号密码来自 `.env`。
- PostgreSQL 默认只绑定 `127.0.0.1`。
- 不默认暴露给局域网或公网。

### Dockerfile

- 多阶段构建。
- Runtime 不包含 Rust Toolchain。
- 非 root 用户运行。
- 不复制 `.env`。
- 不把密钥写入镜像。
- 支持健康检查。
- 支持优雅关闭。

### Caddy

提供示例，不使用真实域名或证书。

EPIC-04 完成前不得声明可以正式公网部署。

---

## 25. Migration 规则

使用 SQLx Migration。

要求：

- Migration 只追加，不修改已提交版本。
- 从空数据库可完整执行。
- 重复启动不会重复破坏数据。
- Migration 失败则服务不能 Ready。
- 生产可关闭启动时自动 Migration，改用独立 Migration Job。
- 测试数据库每次从空库执行全部 Migration。

推荐拆分：

```text
0001_cloud_identity
0002_sync_entities
0003_processed_changes
0004_change_log
0005_snapshots
0006_indexes
```

---

## 26. 测试要求

### 26.1 单元测试

- Config 校验
- Canonical JSON
- Payload Hash
- Scope Hash
- Cursor 编解码
- Cursor 篡改
- Page Token
- Page Token 过期
- Entity Payload 校验
- Error Mapping
- 依赖拓扑排序
- 循环依赖

### 26.2 API 集成测试

#### Health

- Live 正常
- Ready 正常
- 数据库断开 Ready=503

#### Auth

- 无 Token 拒绝
- 错 Token 拒绝
- DEV Token 正常
- Production + DEV_AUTH 启动失败

#### Push

- Create
- Update
- Delete
- 非法 Payload
- 未知 Entity Type
- Base Version Conflict
- Duplicate Change ID
- Change ID 不同内容重用
- 独立 Change 部分成功
- Atomic Group 全成功
- Atomic Group 全回滚
- 缺少依赖
- 循环依赖

#### Pull

- Cursor 顺序
- 分页
- 无重复
- 无遗漏
- Scope 过滤
- Scope 变化
- 跨用户 Cursor

#### Snapshot

- 首次 Snapshot
- 多页
- Keyset Pagination
- 并发 Push
- Snapshot 后 Pull
- Snapshot 过期
- Token 篡改
- 跨用户 Token

#### Tombstone

- Delete 生成 Tombstone
- Pull 返回 Tombstone
- Snapshot 不返回已删除实体
- 重复删除
- 修改已删除实体产生 Conflict

#### 用户隔离

- 两用户相同 Entity ID
- 两用户相同 Change ID
- 两用户数据完全隔离

### 26.3 并发测试

- 两请求同时创建同一 Entity
- 两请求基于同一 Base Version 更新
- 相同 Change ID 并发提交
- 两个 Atomic Group 锁相同 Entity
- Pull 与 Push 并发
- Snapshot 与 Push 并发

期望：

- 只有一个并发更新成功。
- 另一个返回 Conflict 或 Duplicate。
- 不产生重复 Cursor。
- 不出现 Entity 和 Change Log 不一致。

### 26.4 重启持久性

- 服务重启后 Entity 仍存在。
- Processed Change 幂等仍有效。
- Cursor 继续增长。
- Snapshot 正确恢复或过期。

---

## 27. 性能检查

建议基线：

```text
每用户 100,000 Entities
Change Log 1,000,000 Rows
单次 Push 100 Changes
单次 Pull 500 Changes
Snapshot 100,000 Items
```

要求：

- Pull 使用 `(user_id, cursor)` 索引。
- Snapshot 不使用 OFFSET。
- Push 不为每个 Change 新建连接。
- Atomic Group 使用单事务。
- 不一次将用户全部 Entity 加载到内存。
- 使用 `EXPLAIN ANALYZE` 验证关键查询。
- 不要求互联网 SaaS 规模，但不能有明显 N+1 和全表扫描。

---

## 28. 安全验收

- SQL 参数绑定
- CORS 白名单
- Body Size Limit
- Token 日志脱敏
- DATABASE_URL 脱敏
- Cursor HMAC
- Page Token HMAC
- 常量时间签名比较
- 生产禁用 DEV_AUTH
- 用户隔离
- Payload User ID 不可信任
- 客户端不能设置 Server Version
- 未注册 Entity Type 拒绝
- `secret_local_only` 不同步
- 容器非 root
- PostgreSQL 默认仅本机访问
- `.env` 已忽略
- 错误不泄露内部 SQL

---

## 29. 推荐实施阶段

### 阶段 0：前置审计

生成：

```text
docs/epic-03/current-cloud-audit.md
```

只审计，不写云端 API。

### 阶段 1：服务骨架

- Cloud crate
- Config
- Logging
- AppState
- Live/Ready
- Graceful Shutdown
- PostgreSQL Compose

### 阶段 2：数据库 Schema

- Pool
- Migration
- users/devices
- entities
- processed_changes
- change_log
- snapshots
- indexes

### 阶段 3：认证边界

- AuthProvider
- TestAuthProvider
- DevelopmentAuthProvider
- Production 禁用规则

### 阶段 4：校验与 Token

- Registry Validator
- Canonical JSON
- Change Hash
- Cursor
- Page Token
- Scope Hash

### 阶段 5：Push

- Create
- Update
- Delete
- Conflict
- Idempotency
- Atomic Group
- Dependencies

### 阶段 6：Pull

- Cursor
- Scope
- Pagination
- Tombstone
- 用户隔离

### 阶段 7：Snapshot

- Stable Snapshot
- Materialized Items
- Keyset Pagination
- TTL
- Cleanup

### 阶段 8：Docker 与部署

- Dockerfile
- Compose
- Caddy 示例
- PowerShell 脚本
- Smoke Test

### 阶段 9：验收和文档

- 全部测试
- 安全检查
- Completion Report
- EPIC-04/05 接口清单

---

## 30. 推荐提交拆分

```text
docs(cloud): audit epic-03 prerequisites
feat(cloud): add standalone axum service
feat(cloud): add postgres migrations
feat(cloud): add authentication provider boundary
feat(sync-server): add entity validation and storage
feat(sync-server): implement idempotent push
feat(sync-server): implement pull and cursor
feat(sync-server): implement snapshots
test(sync-server): add concurrency and isolation tests
build(cloud): add docker and compose
docs(cloud): complete epic-03 documentation
```

不得将整个 EPIC 放在一个提交中。

---

## 31. Definition of Done

- [ ] EPIC-02 Contract 能编译并通过测试
- [ ] 独立 `lifetrace-cloud` 服务存在
- [ ] 不依赖 Tauri 和 rusqlite
- [ ] PostgreSQL 可通过 Compose 启动
- [ ] SQLx Migration 从空库成功
- [ ] Live 和 Ready 正常
- [ ] Config 强类型校验
- [ ] Production 启用 DEV_AUTH 会拒绝启动
- [ ] AuthProvider 可由 EPIC-04 替换
- [ ] Capabilities 与 Registry/Config 一致
- [ ] Payload 入库前经过 Contract 校验
- [ ] 未注册 Entity Type 被拒绝
- [ ] 客户端不能伪造用户和 Server Version
- [ ] Cursor 由服务端分配
- [ ] Cursor 与用户和 Scope 绑定
- [ ] Push Create/Update/Delete 通过
- [ ] Base Version Conflict 通过
- [ ] Change ID 幂等通过
- [ ] Change ID 重用被拒绝
- [ ] Atomic Group 全成全败
- [ ] Tombstone 可 Pull
- [ ] Pull 严格按 Cursor 升序
- [ ] Pull 分页无重复无遗漏
- [ ] Snapshot 稳定
- [ ] Snapshot 使用 Keyset Pagination
- [ ] Snapshot 后 Pull 不漏数据
- [ ] 用户隔离测试通过
- [ ] 并发更新测试通过
- [ ] 重启后幂等和数据仍存在
- [ ] Docker 镜像可构建
- [ ] 容器非 root
- [ ] CORS 不默认允许全部来源
- [ ] 日志不记录 Token 和完整 Payload
- [ ] Cloud 测试通过
- [ ] Contract 测试通过
- [ ] 桌面现有测试通过
- [ ] 全仓库构建通过
- [ ] 文档完整
- [ ] 未完成正式登录时没有公开生产部署

---

# 32. 可直接交给 Agent 的完整提示词

```text
你现在负责完成 LifeTrace 仓库的 EPIC-03：独立云端后端服务。

仓库：
zhouxingxing1279/LifeTrace

当前桌面技术栈：
Tauri 2 + React/Vite + Rust/Axum + rusqlite + SQLite。

云端目标：
Rust + Axum + PostgreSQL + SQLx + Docker。

重要前提：
EPIC-01 已完成。
EPIC-02 公共契约和同步协议必须完成后，EPIC-03 才能正式实现。
你必须检查代码，不能假定 EPIC-02 已完成。

开始前必须阅读：

- README.md
- package.json
- 根 Cargo.toml（如果存在）
- src-tauri/Cargo.toml
- src-tauri/src/server.rs
- src-tauri/src/database/**
- docs/epic-01/**
- docs/epic-02/**
- crates/lifetrace-contracts/**
- contracts/openapi/**
- contracts/json-schema/**

搜索：

- cloud
- postgres
- sqlx
- docker
- sync
- cursor
- push
- pull
- snapshot
- tombstone
- change_id
- server_version
- entity_type
- AuthProvider
- DATABASE_URL

第一轮只能完成“阶段 0：前置审计”。

生成：

docs/epic-03/current-cloud-audit.md

审计必须说明：

1. EPIC-01 当前状态
2. EPIC-02 当前完成状态
3. Contract crate 是否能编译
4. Push/Pull/Snapshot DTO 是否稳定
5. Entity Registry 是否完整
6. 是否存在无稳定 ID 的 Entity
7. localVersion 与 serverVersion 是否分离
8. 当前 Cargo Workspace
9. 是否已有云端/PostgreSQL/Docker 代码
10. 实施阻塞项
11. 本方案需要根据实际代码调整的内容
12. 下一阶段预计修改文件

如果 EPIC-02 未完成，停止并输出阻塞报告。不得定义第二套同步协议。

任务边界：

EPIC-03 必须实现：

- 独立 services/lifetrace-cloud
- PostgreSQL 和 SQLx Migration
- 配置
- Live/Ready
- AuthProvider
- DEV_AUTH
- Capabilities
- Push
- Pull
- Snapshot
- Tombstone
- Conflict
- Change ID 幂等
- Atomic Group
- 用户隔离
- Cursor/Page Token
- Docker/Compose
- 测试和文档

EPIC-03 不实现：

- 正式注册登录
- 密码哈希
- JWT 签发
- Refresh Token
- 正式设备管理
- 客户端 Sync Outbox
- 客户端后台 Worker
- Android App
- 文件二进制上传
- S3/MinIO
- 邮件和 AI 服务

云端必须直接依赖 crates/lifetrace-contracts。
不得重新定义 Sync DTO、Entity DTO、ErrorCode、Cursor 或 ServerVersion。

PostgreSQL 使用通用 Entity Store：

- cloud_users
- cloud_devices
- sync_entities
- sync_processed_changes
- sync_change_log
- sync_snapshots
- sync_snapshot_items

sync_entities 保存完整、经过 Contract 校验的 JSONB Entity Payload。
不得接收任意 JSON。
不得接收未注册 Entity Type。
不得同步 secret_local_only 数据。

认证：

定义 AuthProvider 和 AuthenticatedPrincipal。
本 Epic 只实现 TestAuthProvider 和 DevelopmentAuthProvider。
Development Auth 使用固定 Bearer Token。
禁止使用任意 X-User-Id。
Production 启用 DEV_AUTH 时必须拒绝启动。
正式登录由 EPIC-04 实现。

Push：

认证
→ 版本校验
→ 批量校验
→ Entity Registry 校验
→ Payload 校验
→ Canonical Hash
→ Atomic Group
→ PostgreSQL Transaction
→ Entity Lock
→ Base Version 校验
→ Change Log
→ Entity Store
→ Processed Change

相同 userId + changeId + 相同 Hash：
返回 Duplicate，不重复执行。

相同 userId + changeId + 不同 Hash：
返回 LIFETRACE_CHANGE_ID_REUSE。

Create serverVersion=1。
Update/Delete serverVersion=current+1。
不得由客户端设置 serverVersion。

Conflict：

baseServerVersion != currentServerVersion 时返回 Conflict。
返回服务器当前 Payload 或 Tombstone。
不得 Last Write Wins。
不得按客户端时间决定。

Delete：

生成 Tombstone。
不得硬删除。
Pull 必须返回 Tombstone。
Snapshot 不返回已删除 Entity。

Atomic Group：

同一 group 在一个 PostgreSQL 事务中。
全部成功或全部回滚。
固定锁顺序。
依赖拓扑排序。
循环依赖拒绝。

Cursor：

内部 BIGINT。
线路使用 HMAC 签名 opaque token。
绑定 userId、scopeHash、position 和 protocolVersion。
跨用户、跨 Scope、篡改均拒绝。

Pull：

严格按 Cursor 升序。
所有查询带 user_id。
支持 Scope 和分页。
无重复无遗漏。
不得按 updatedAt 排序。

Snapshot：

使用 REPEATABLE READ。
物化到 sync_snapshot_items。
使用 Keyset Pagination。
Page Token 签名并有 TTL。
完成后返回 snapshotCursor。
客户端随后从 snapshotCursor Pull。

用户隔离：

测试两用户相同 Entity ID、相同 Change ID、不同 Cursor 和 Snapshot。
任何查询不能只凭 entity_id/change_id/snapshot_id。

配置至少包含：

- LIFETRACE_ENV
- LIFETRACE_BIND_ADDRESS
- DATABASE_URL
- DATABASE_MIN_CONNECTIONS
- DATABASE_MAX_CONNECTIONS
- MIGRATION_ON_STARTUP
- REQUEST_BODY_LIMIT_BYTES
- PUSH_MAX_CHANGES
- PULL_MAX_CHANGES
- SNAPSHOT_MAX_PAGE_SIZE
- MAXIMUM_ATOMIC_GROUP_SIZE
- CURSOR_SIGNING_KEY
- PAGE_TOKEN_SIGNING_KEY
- CORS_ALLOWED_ORIGINS
- DEV_AUTH_ENABLED
- DEV_AUTH_USER_ID
- DEV_AUTH_DEVICE_ID
- DEV_AUTH_TOKEN
- SNAPSHOT_TTL_SECONDS
- RUST_LOG

API：

GET  /health/live
GET  /health/ready
GET  /api/v1/meta/version
GET  /api/v1/sync/capabilities
POST /api/v1/sync/push
POST /api/v1/sync/pull
POST /api/v1/sync/snapshot

Docker：

- PostgreSQL 固定 Major Version
- 不使用 latest
- PostgreSQL 默认只绑定 127.0.0.1
- Cloud 容器非 root
- 不复制 .env
- 提供 Compose、Caddy 示例和 PowerShell 脚本

测试必须覆盖：

- Config
- Canonical JSON
- Hash
- Cursor
- Token 篡改
- Live/Ready
- Auth
- Production 禁用 DEV_AUTH
- Push Create/Update/Delete
- Conflict
- Duplicate Change
- Change ID Reuse
- Atomic Group
- Dependency
- Pull Order/Pagination
- Tombstone
- Snapshot
- Snapshot 后 Pull
- 用户隔离
- 并发更新
- 服务重启持久性

分阶段执行：

0. 前置审计
1. 服务骨架
2. PostgreSQL Schema
3. 认证边界
4. Payload/Hash/Cursor
5. Push
6. Pull
7. Snapshot
8. Docker/Deployment
9. 验收和文档

每个阶段单独提交并测试。
不要一次完成所有修改后再测试。

最终生成：

docs/epic-03/completion-report.md

包含：

- 修改文件
- Workspace 变化
- PostgreSQL Migration
- 数据表与索引
- API
- AuthProvider 边界
- Push/Pull/Snapshot
- 幂等和 Conflict
- 测试结果
- Docker Smoke Test
- 安全检查
- 已知问题
- EPIC-04 接口点
- EPIC-05 接口点

在 Definition of Done 全部满足前，不要声称 EPIC-03 已完成。
```

---

## 33. 推荐给 Agent 的第一轮指令

不要一开始就让 Agent 实现全部云端功能。

第一轮只发送：

```text
请阅读：

- docs/epic-03/implementation-plan.md
- docs/epic-01/**
- docs/epic-02/**
- crates/lifetrace-contracts/**
- 当前 Cargo Workspace

本轮只执行 EPIC-03 的“阶段 0：前置审计”。

生成：

docs/epic-03/current-cloud-audit.md

必须确认：

- EPIC-02 是否真正完成
- Contract 是否可编译
- Entity Registry 是否可用
- Push/Pull/Snapshot DTO 是否稳定
- 是否存在无稳定 ID 的同步实体
- 当前是否已有 Cloud/PostgreSQL/Docker 代码
- 本方案需要调整的内容
- 阻塞项
- 下一阶段预计修改文件

本轮不得：

- 创建生产同步 API
- 定义第二套同步 DTO
- 实现正式登录
- 实现客户端同步 Worker
- 提前执行阶段 1

完成审计后停止。
```
