# LifeTrace EPIC-05：Windows 客户端同步核心——Agent 具体实施方案

> 目标仓库：`zhouxingxing1279/LifeTrace`  
> 目标分支基线：`main`  
> 建议开发分支：`agent/epic-05-windows-sync-core`  
> 前置依赖：EPIC-02 公共领域与同步契约、EPIC-03 PostgreSQL 云端服务、EPIC-04 账号认证与设备管理已经完成。  
> 当前实施范围：**仅实现 Windows/Tauri 客户端同步，不实现 Android、Kotlin、Room、WorkManager 或 Android App。**  
> 核心原则：**本地优先、离线可写、业务与 Outbox 同事务、服务端用户隔离、显式冲突、删除墓碑、幂等重试、凭据安全、同步失败不影响本地使用。**

---

# 1. EPIC-05 最终目标

EPIC-05 完成后，LifeTrace Windows 客户端应具备以下完整同步闭环：

```text
用户在 Windows LifeTrace 中修改数据
        ↓
写入本地 SQLite 业务表
        +
同一事务写入 Sync Outbox
        ↓
后台同步引擎 Push
        ↓
LifeTrace Cloud PostgreSQL
        ↓
其他设备变更通过 Cursor Pull
        ↓
事务性应用到本地 SQLite
```

必须实现：

1. 未登录时继续完整使用本地功能。
2. 断网时可创建、修改和删除业务数据。
3. 本地业务写入与 Outbox 在同一个 SQLite 事务中提交。
4. 网络恢复后自动上传未同步变更。
5. 重复 Push 不产生重复数据。
6. 基于服务端 Cursor 增量 Pull。
7. Pull 页面应用失败时 Cursor 不得前进。
8. Access Token 过期时自动刷新。
9. Refresh Token 继续保存在 Windows Credential Manager。
10. 冲突必须持久化，不允许静默覆盖。
11. 删除墓碑不能被离线旧数据复活。
12. 新安装或本地数据库重建后可通过 Snapshot 恢复。
13. 用户可以查看同步状态、待上传数量和冲突数量。
14. 应用重启、Cloud 重启、网络中断后均可继续同步。

---

# 2. 本阶段明确不实现

以下内容不属于本轮 EPIC-05：

```text
Android App
Kotlin 同步 SDK
Room 数据库
Android WorkManager
Android Credential Storage
Android UI
Finance Android App
Notes Android App
English Android App
Habits Android App
```

本轮可以保留：

- 稳定的公共同步 Contract；
- 与平台无关的协议测试 Fixture；
- 清晰的同步状态机；
- 后续 Android 可参考的行为规范。

但不得因为未来 Android 复用而引入 JNI、UniFFI 或 Rust Mobile Runtime。

---

# 3. 开始实施前的强制审计

Agent 必须先读取：

```text
docs/LifeTrace_Complete_Roadmap_v2.md
docs/LifeTrace_EPIC02_Agent_Implementation_Plan.md
docs/LifeTrace_EPIC03_Agent_Implementation_Plan.md
docs/epic-04/LifeTrace_EPIC04_Agent_Implementation_Plan.md
docs/epic-04/completion-report.md

crates/lifetrace-contracts/src/sync/**
crates/lifetrace-contracts/src/domain/**
crates/lifetrace-contracts/src/auth/**

services/lifetrace-cloud/src/**
services/lifetrace-cloud/migrations/**
services/lifetrace-cloud/tests/**

src-tauri/src/database/**
src-tauri/src/server/**
src-tauri/src/cloud_auth.rs
src-tauri/src/lib.rs

src/**
```

必须确认：

- Push、Pull、Snapshot 的真实路径和 DTO；
- `change_id`、`base_version`、`server_version`、Cursor 的现有定义；
- 服务端批量限制；
- Snapshot 的分页方式；
- Conflict 返回格式；
- Entity Registry 中可同步的实体；
- Access Token Refresh 接口；
- 当前本地 Repository 的所有写入入口；
- 当前所有业务表中的 `user_id` 使用方式；
- 前端当前云账号和 Token 管理方式。

审计结果写入：

```text
docs/epic-05/precondition-report.md
```

报告必须包括：

```text
现有能力
缺失能力
需要修改的数据库表
需要接入 Outbox 的业务写入入口
协议是否足够
用户归属模型问题
主要风险
```

未经审计不得直接创建同步 Worker。

---

# 4. 用户归属模型

EPIC-05 必须先明确以下四个不同概念：

```text
Local Profile
本地 SQLite 中的数据档案

Cloud User
EPIC-04 创建的云端账号

App ID
lifetrace-desktop 等应用身份

Device ID
当前安装实例或设备身份
```

禁止使用一个含义模糊的 `user_id` 同时表示这四个概念。

## 4.1 云端归属

云端业务与同步数据的所有者必须来自：

```text
Access Token
→ AuthenticatedPrincipal
→ principal.user_id
```

客户端不得通过请求参数决定云端 `user_id`。

所有云端查询和写入必须包含：

```sql
WHERE user_id = $authenticated_user_id
```

Push 请求中的 Entity Payload 即使包含 `userId`，也不得作为云端所有权依据。

## 4.2 本地归属

Windows 本地数据库使用稳定的本地 Profile：

```text
local_profile_id
```

本地 Profile 可以处于两种状态：

```text
local
尚未绑定云端账号

cloud
已经绑定一个 cloud_user_id
```

推荐新增表：

```sql
CREATE TABLE local_profiles (
    id TEXT PRIMARY KEY,
    profile_type TEXT NOT NULL
        CHECK (profile_type IN ('local', 'cloud')),

    cloud_user_id TEXT,
    display_name TEXT NOT NULL,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    CHECK (
        (profile_type = 'local' AND cloud_user_id IS NULL)
        OR
        (profile_type = 'cloud' AND cloud_user_id IS NOT NULL)
    ),

    UNIQUE (cloud_user_id)
);
```

## 4.3 兼容现有 `user_id`

现有本地业务表中的：

```text
user_id
```

本轮不强制全部重命名，以避免大规模破坏性 Migration。

但必须统一解释为：

```text
本地 user_id = local_profiles.id
```

它不等于 Cloud User ID。

代码中必须使用明确类型：

```rust
pub struct LocalProfileId(pub String);
pub struct CloudUserId(pub String);
```

禁止两个 ID 继续使用无类型的普通字符串混用。

## 4.4 历史数据迁移

现有：

```text
user_id = 'local'
```

必须迁移为稳定 UUID。

Migration 流程：

```text
创建默认 local_profiles 记录
→ 生成稳定 LocalProfileId
→ 将所有业务表中的 'local' 更新为该 ID
→ 保存当前 Active Profile
```

必须覆盖：

```text
finance_accounts
transaction_categories
transactions
activities
activity_logs
daily_reviews
notes
note_folders
note_tags
english_articles
english_learning_records
vocabulary
workout 相关表
其他 Entity Registry 中可同步实体
```

迁移完成后不允许新增：

```text
local
default
anonymous
guest
```

等字符串型占位用户。

---

# 5. 登录后的本地数据处理

用户首次登录云账号时，不得自动上传全部历史本地数据。

必须提供两个选择：

## 5.1 绑定当前本地档案

```text
将当前本地数据绑定到该云账号
```

执行：

```sql
UPDATE local_profiles
SET profile_type = 'cloud',
    cloud_user_id = ?,
    updated_at = ?
WHERE id = ?;
```

绑定完成后：

- 为本地已有实体生成初始 Outbox；
- 使用实体当前版本作为本地版本；
- 首次上传前执行 Snapshot 或 Cloud 空状态检查；
- 不允许该 Profile 再绑定其他 Cloud User。

## 5.2 创建新的云端档案

```text
保留当前本地档案
创建新的空白云端档案
```

执行：

- 新建 `local_profiles`；
- 绑定登录 Cloud User；
- 从 Cloud Snapshot 初始化；
- 原本地数据继续保留在旧 Profile；
- 两个 Profile 之间不得自动合并。

## 5.3 退出登录

退出只执行：

```text
停止同步
清除 Access Token
清除 Windows Credential Manager 中的 Refresh Token
将同步状态切换为 AuthRequired 或 LocalOnly
```

不得删除本地业务数据。

---

# 6. 推荐代码结构

新增纯 Rust 同步核心：

```text
crates/
└── lifetrace-sync-client/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── engine.rs
        ├── state.rs
        ├── push.rs
        ├── pull.rs
        ├── snapshot.rs
        ├── retry.rs
        ├── conflict.rs
        ├── error.rs
        ├── traits.rs
        └── testkit.rs
```

该 crate 可以依赖：

```text
lifetrace-contracts
serde
serde_json
async-trait
tokio
thiserror
tracing
```

不得依赖：

```text
Tauri
rusqlite
Windows API
React
Axum
SQLx
```

Windows 适配层新增：

```text
src-tauri/src/sync/
├── mod.rs
├── manager.rs
├── sqlite_store.rs
├── entity_applier.rs
├── http_transport.rs
├── auth_session.rs
├── scheduler.rs
├── commands.rs
└── status.rs
```

职责划分：

```text
lifetrace-sync-client
负责同步状态机和协议行为

src-tauri/src/sync
负责 SQLite、HTTP、Credential Manager、Tauri 生命周期
```

---

# 7. 本地数据库 Migration

新增：

```text
src-tauri/src/database/migrations/m0007_sync_core.rs
```

并在 Migration Registry 中注册。

## 7.1 local_profiles

保存本地档案与 Cloud User 的绑定关系。

## 7.2 app_settings 或 active_profile

保存当前激活的 Profile：

```sql
CREATE TABLE app_profile_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    active_profile_id TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (active_profile_id)
        REFERENCES local_profiles(id)
);
```

## 7.3 sync_outbox

推荐结构：

```sql
CREATE TABLE sync_outbox (
    change_id TEXT PRIMARY KEY,

    profile_id TEXT NOT NULL,
    app_id TEXT NOT NULL,
    device_id TEXT NOT NULL,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL
        CHECK (operation IN ('upsert', 'delete')),

    base_version INTEGER,
    payload_json TEXT,
    atomic_group_id TEXT,

    status TEXT NOT NULL
        CHECK (status IN (
            'pending',
            'in_flight',
            'conflict',
            'blocked',
            'confirmed',
            'dead_letter'
        )),

    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT,

    lease_owner TEXT,
    lease_expires_at TEXT,

    last_error_code TEXT,
    last_error_message TEXT,

    server_version INTEGER,
    confirmed_cursor TEXT,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    FOREIGN KEY (profile_id)
        REFERENCES local_profiles(id)
        ON DELETE CASCADE
);
```

必要索引：

```sql
(profile_id, status, next_attempt_at)
(profile_id, entity_type, entity_id)
(profile_id, lease_expires_at)
```

## 7.4 sync_state

```sql
CREATE TABLE sync_state (
    profile_id TEXT NOT NULL,
    app_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    scope_key TEXT NOT NULL,

    cursor TEXT,
    snapshot_id TEXT,
    snapshot_page_token TEXT,
    snapshot_status TEXT,

    protocol_version INTEGER,
    schema_version INTEGER,

    last_attempt_at TEXT,
    last_success_at TEXT,
    last_error_code TEXT,
    last_error_message TEXT,

    PRIMARY KEY (
        profile_id,
        app_id,
        device_id,
        scope_key
    ),

    FOREIGN KEY (profile_id)
        REFERENCES local_profiles(id)
        ON DELETE CASCADE
);
```

禁止全局只保存一个 Cursor。

## 7.5 sync_conflicts

```sql
CREATE TABLE sync_conflicts (
    conflict_id TEXT PRIMARY KEY,

    profile_id TEXT NOT NULL,
    change_id TEXT,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,

    base_version INTEGER,
    server_version INTEGER,

    local_payload_json TEXT,
    remote_payload_json TEXT,

    conflict_kind TEXT NOT NULL,
    resolution_status TEXT NOT NULL
        CHECK (resolution_status IN (
            'unresolved',
            'accepted_remote',
            'kept_local',
            'discarded'
        )),

    created_at TEXT NOT NULL,
    resolved_at TEXT,

    FOREIGN KEY (profile_id)
        REFERENCES local_profiles(id)
        ON DELETE CASCADE
);
```

## 7.6 sync_metadata

```sql
CREATE TABLE sync_metadata (
    profile_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,

    server_version INTEGER,
    last_server_cursor TEXT,
    origin_device_id TEXT,
    payload_hash TEXT,

    sync_status TEXT NOT NULL,
    deleted_at TEXT,
    updated_at TEXT NOT NULL,

    PRIMARY KEY (
        profile_id,
        entity_type,
        entity_id
    ),

    FOREIGN KEY (profile_id)
        REFERENCES local_profiles(id)
        ON DELETE CASCADE
);
```

---

# 8. Repository 归属上下文

所有本地 Repository 必须通过统一上下文访问数据：

```rust
pub struct OwnerContext {
    pub profile_id: LocalProfileId,
    pub cloud_user_id: Option<CloudUserId>,
    pub app_id: AppId,
    pub device_id: DeviceId,
}
```

禁止新增：

```rust
list_transactions(connection)
get_note(connection, id)
```

应改为：

```rust
list_transactions(connection, owner, filter)
get_note(connection, owner, id)
```

所有 SQL 必须包含本地归属条件：

```sql
WHERE user_id = ?profile_id
```

前端不得自由传入任意用户 ID。

当前 Profile 应由 Tauri `AppState` 维护：

```rust
pub struct DesktopAppState {
    pub active_profile: ActiveProfileState,
    pub cloud_session: CloudSessionState,
    pub sync_manager: SyncManager,
}
```

---

# 9. 业务写入与 Outbox 同事务

所有本地业务写入必须符合：

```text
BEGIN IMMEDIATE

写入业务实体
更新本地 version
更新 sync_metadata
写入或合并 sync_outbox

COMMIT
```

如果 Outbox 写入失败，业务修改必须回滚。

禁止：

```text
写入业务实体
COMMIT
异步创建 Outbox
```

禁止由 React 前端创建 Outbox。

## 9.1 Outbox 合并规则

同一 Profile、Entity Type、Entity ID 的未确认变更应合并：

```text
Create + Update
→ 保留最初 Change，Payload 更新为最新实体

Create + Delete，且从未上传
→ 删除 Pending Outbox
→ 删除或保留本地软删除记录
→ 不发送无意义 Delete

Update + Update
→ 保留最早 base_version
→ 使用最新 Payload

Update + Delete
→ 转换为 Delete
→ 保留正确 base_version

Conflict / Blocked
→ 不允许被普通合并静默覆盖
```

已确认 Change 不得被重新修改。

---

# 10. 远端变更应用模式

必须明确区分：

```text
Local Write
Remote Apply
Migration
Import
Conflict Resolution
```

远端 Pull 应用必须使用：

```rust
WriteOrigin::RemoteSync
```

`RemoteSync` 模式：

- 写入业务表；
- 更新 `sync_metadata`；
- 不生成新的 Outbox；
- 不触发普通本地写入同步防抖；
- 保留服务端版本和 Cursor。

否则会出现无限同步回环：

```text
Pull
→ 本地写入
→ 新 Outbox
→ Push
→ 再次 Pull
```

---

# 11. Push 设计

Push 使用现有：

```text
POST /api/v1/sync/push
```

不得重新定义第二套 DTO。

流程：

```text
领取到期 Pending Outbox
→ 写入租约
→ 按批量限制分组
→ 保留 Atomic Group
→ 获取 Access Token
→ 调用 Cloud
→ 逐项处理结果
→ 更新 Outbox、Metadata、Conflict
```

## 11.1 租约

领取时：

```text
pending
→ in_flight
```

同时写入：

```text
lease_owner
lease_expires_at
```

应用崩溃后，过期 `in_flight` 必须可重新领取。

## 11.2 错误分类

```text
401
→ Single-Flight Refresh
→ 原请求只重试一次
→ 再次 401 后进入 AuthRequired

403
→ blocked
→ 提示 Scope 或 App Grant 不足

409
→ 保存 sync_conflicts

413
→ 缩小 Batch

429
→ 尊重 Retry-After

网络错误、超时、5xx
→ 指数退避

协议版本不兼容
→ 停止自动同步
→ 提示升级客户端

未注册 Entity Type
→ dead_letter 或 blocked
→ 不得无限重试
```

## 11.3 指数退避

建议：

```text
初始：2 秒
上限：5 分钟
随机抖动：±20%
```

测试必须注入 Clock 和随机源，不得真实等待数分钟。

---

# 12. Token 自动刷新

Windows 凭据规则：

```text
Access Token
只存在内存

Refresh Token
只存 Windows Credential Manager

密码
不在客户端持久化
```

必须实现 Single-Flight Refresh：

```text
多个请求同时遇到 401
→ 只允许一个 Refresh 请求
→ 其他请求等待相同结果
```

Refresh Token 轮换成功：

1. 先安全保存新的 Refresh Token；
2. 更新内存 Access Token；
3. 清除旧的内存状态；
4. 唤醒等待请求。

不得将 Access Token 或 Refresh Token 写入：

```text
SQLite
localStorage
普通配置文件
日志
错误消息
同步状态表
```

---

# 13. Pull 设计

Pull 使用现有：

```text
GET /api/v1/sync/pull
```

流程：

```text
读取 Profile 对应 Cursor
→ 请求一页
→ BEGIN
→ 应用整页实体
→ 处理墓碑
→ 更新 sync_metadata
→ 确认本设备已接受变更
→ 最后更新 Cursor
→ COMMIT
→ 请求下一页
```

关键规则：

1. Cursor 必须最后更新。
2. 页面任意实体失败，整页回滚。
3. 重启后可以重新请求同一页。
4. 不依赖客户端本地时间排序。
5. Remote Apply 不得产生 Outbox。
6. 同一 `change_id` 应识别为本设备提交确认。
7. 有未确认本地修改时不得直接覆盖。

---

# 14. 冲突处理

发生以下情况时保存冲突：

```text
远端实体版本更新
+
本地存在未确认变更
```

或者：

```text
远端已经删除
+
本地仍有旧版本修改
```

第一版提供两种解决方式：

## 14.1 接受云端

```text
应用远端 Payload 或 Tombstone
清除相关 Pending Outbox
更新 sync_metadata
标记 Conflict 为 accepted_remote
```

## 14.2 保留本地

```text
使用最新 server_version 作为 base_version
生成新的 change_id
创建新 Outbox
保留本地 Payload
标记旧 Conflict 为 kept_local
```

禁止使用：

```text
updated_at 较新者胜出
客户端时间较新者胜出
```

---

# 15. 删除墓碑

本地删除：

```text
业务记录软删除
+
生成 Delete Outbox
```

远端 Tombstone Pull：

```text
写入本地 deleted_at
更新 server_version
更新 sync_metadata
```

必须保证：

```text
设备 A 删除实体
设备 B 离线保留旧版本
设备 B 上线
→ 不得自动重新创建该实体
```

如果设备 B 有未同步修改：

```text
进入 Delete Conflict
```

只有用户明确选择“保留本地”时，才能以新的 Change 和最新 Base Version 重建。

---

# 16. Snapshot 设计

Snapshot 用于：

```text
新 Profile
新安装
本地没有 Cursor
Cursor 失效
用户主动重新初始化
本地同步状态损坏
```

流程：

```text
创建本地备份
→ 标记 InitializingSnapshot
→ 分页下载 Snapshot
→ 保存 Snapshot ID 和 Page Token
→ 写入 Staging
→ 校验 Snapshot
→ 事务性切换
→ 重新处理本地 Pending Outbox
→ 保存 Snapshot Cursor
→ 转入 Pull
```

推荐新增临时表：

```text
snapshot_staging_entities
snapshot_staging_metadata
```

Snapshot 不得直接一页一页覆盖正式业务表，否则中断时会产生半完成状态。

## 16.1 Pending 本地数据

Snapshot 不得覆盖尚未上传的本地修改。

策略：

```text
保存 Pending Outbox
→ 应用 Snapshot 权威副本
→ 对 Pending Change 进行 Rebase
→ 无法 Rebase 时生成 Conflict
```

## 16.2 重新初始化

“从云端重新初始化”必须：

- 二次确认；
- 自动备份 SQLite；
- 显示未上传数据数量；
- 有未上传数据时不得默认丢弃；
- 完成后验证记录数量和 Cursor。

---

# 17. 同步状态机

定义：

```text
Disabled
LocalOnly
AuthRequired
Idle
InitializingSnapshot
Pushing
Pulling
UpToDate
Offline
Backoff
Conflict
Error
```

状态机要求：

- 同一 Profile 只允许一轮同步运行；
- 手动同步与后台同步共享同一锁；
- 状态变化可由前端订阅；
- 错误必须脱敏；
- 不得展示 Token 或完整业务 Payload。

---

# 18. Windows 调度

必须支持：

## 18.1 应用启动

登录状态有效时：

```text
启动后延迟短时间
→ 执行同步
```

## 18.2 本地变更

业务写入成功后：

```text
发送 SyncRequested 事件
→ 2 秒防抖
→ Push/Pull
```

## 18.3 周期同步

建议默认：

```text
5 分钟
```

必须可配置，不能高频轮询。

## 18.4 网络恢复

网络失败进入 `Offline` 或 `Backoff`。

后续周期检查成功后恢复同步。

## 18.5 手动同步

前端提供：

```text
立即同步
```

调用与后台任务相同的同步流程，不允许并发启动另一套逻辑。

## 18.6 登录与退出

```text
登录成功
→ 绑定或选择 Profile
→ 启动同步

退出
→ 停止调度
→ 清除凭据
→ 保留本地数据
```

---

# 19. Tauri Command

建议新增：

```rust
sync_status
sync_now
sync_retry
sync_list_conflicts
sync_resolve_conflict
sync_initialize_from_cloud
profile_list
profile_get_active
profile_switch
profile_bind_cloud_user
profile_create_cloud_profile
```

前端不得传入任意 Cloud User ID。

云端 User ID 必须来自当前认证 Session。

---

# 20. 前端同步状态 UI

本轮只增加精简同步入口，不重构整个 UI。

显示：

```text
本地模式
已登录
正在上传
正在下载
同步完成
离线
需要重新登录
待上传数量
冲突数量
最后成功时间
最近错误
```

操作：

```text
立即同步
重试
查看冲突
切换本地档案
绑定当前数据
从云端重新初始化
```

首次登录时必须出现数据归属选择：

```text
将当前本地数据绑定到该账号

创建新的空白云端档案
```

不得默认选择并自动上传。

---

# 21. 测试要求

## 21.1 Local Profile

必须测试：

```text
'local' 历史数据迁移为稳定 Profile UUID
本地 Profile 可不绑定 Cloud User
一个 Profile 只能绑定一个 Cloud User
同一 Cloud User 不得绑定多个本地 Profile
切换 Profile 后数据完全隔离
退出登录不删除本地数据
登录其他用户不会读取原用户数据
```

## 21.2 Outbox 事务

必须测试：

```text
业务写入成功，Outbox 同时存在
业务写入失败，Outbox 不存在
Outbox 写入失败，业务写入回滚
应用重启后 Pending Outbox 仍存在
过期 in_flight 可重新领取
```

## 21.3 Push

必须测试：

```text
批量 Push
幂等重发
部分成功
部分 Conflict
401 自动刷新
并发 401 只刷新一次
429 Retry-After
5xx 指数退避
权限不足转 blocked
未知实体不无限重试
```

## 21.4 Pull

必须测试：

```text
Cursor 分页
页面事务性应用
实体失败时 Cursor 不前进
Remote Apply 不生成 Outbox
本设备 Change 被确认
其他设备变更正确应用
```

## 21.5 Tombstone

必须测试：

```text
删除同步到第二个客户端
离线客户端旧数据不能复活
本地修改与远端删除产生 Conflict
Snapshot 保留删除语义
```

## 21.6 Snapshot

必须测试：

```text
新 Profile 初始化
分页断点续传
中断恢复
Staging 校验
正式切换
Pending Outbox Rebase
Snapshot 后继续 Pull
```

## 21.7 多用户隔离

使用两个真实测试用户：

```text
User A
User B
```

验证：

- A 不能 Pull 到 B 数据；
- A 的 Cursor 不影响 B；
- A 的 Snapshot 不包含 B 数据；
- 切换本地 Profile 后 Repository 查询正确；
- 前端无法构造任意用户 ID 读取其他数据。

---

# 22. Cloud 端到端测试

使用：

```text
PostgreSQL
LifeTrace Cloud
Windows Client A
Windows Client B
```

完成：

1. A 离线创建记录；
2. A Push；
3. B Pull；
4. A 重复 Push；
5. A、B 同时修改；
6. Conflict 持久化；
7. A 删除；
8. B 离线修改旧记录；
9. B 上线后不能复活；
10. 新 Client C 通过 Snapshot 恢复；
11. Access Token 过期自动刷新；
12. Cloud 重启后继续同步；
13. Client 重启后继续同步；
14. User A 与 User B 数据隔离。

---

# 23. CI

新增：

```text
.github/workflows/epic05-windows-sync.yml
```

包含：

```text
Rust fmt
lifetrace-contracts tests
lifetrace-sync-client tests
Tauri SQLite integration tests
PostgreSQL + Cloud E2E
Windows cargo check
Windows Tauri check
Clippy -D warnings
Frontend typecheck
Frontend tests
Frontend production build
Cloud regression tests
Docker build
```

本轮不增加：

```text
Android SDK
Gradle
Kotlin
Room
WorkManager
```

---

# 24. 文档产物

必须创建：

```text
docs/epic-05/precondition-report.md
docs/epic-05/design.md
docs/epic-05/user-ownership.md
docs/epic-05/sync-state-machine.md
docs/epic-05/test-matrix.md
docs/epic-05/completion-report.md
```

其中 `user-ownership.md` 必须明确：

```text
LocalProfileId
CloudUserId
AppId
DeviceId
```

四者之间的关系。

---

# 25. 建议实施阶段

## Phase 0：审计

完成前置报告和所有写入入口清单。

## Phase 1：用户归属 Migration

完成：

```text
local_profiles
active_profile
历史 'local' 数据迁移
OwnerContext
Profile 隔离测试
```

## Phase 2：同步数据库

完成：

```text
sync_outbox
sync_state
sync_conflicts
sync_metadata
Snapshot Staging
```

## Phase 3：同步核心

完成：

```text
状态机
Retry
Lease
Push/Pull/Snapshot 抽象
错误分类
```

## Phase 4：业务写入接入 Outbox

按领域逐个接入：

```text
Finance
Habits
Reviews
Notes
English
Workouts
```

不得使用未经审查的全局字符串替换。

## Phase 5：Push 与 Token Refresh

完成：

```text
HTTP Transport
Credential Provider
Single-Flight Refresh
Push Worker
```

## Phase 6：Pull、Tombstone、Conflict

完成增量下载和冲突解决。

## Phase 7：Snapshot

完成新 Profile 初始化和重新初始化。

## Phase 8：调度与 UI

完成后台调度、状态订阅和用户界面。

## Phase 9：E2E、CI 与文档

完成真实 PostgreSQL 多客户端测试和 Completion Report。

---

# 26. 完成定义

只有以下全部满足，才能声明 EPIC-05 完成：

- [ ] 历史 `'local'` 用户已迁移为稳定 Local Profile；
- [ ] Local Profile 与 Cloud User 的关系清晰；
- [ ] 首次登录不会自动上传本地数据；
- [ ] 所有本地业务读取按 Profile 隔离；
- [ ] 所有业务写入与 Outbox 同事务；
- [ ] Push 支持幂等、租约、重试和部分成功；
- [ ] Access Token 自动刷新；
- [ ] Refresh Token 只存 Windows Credential Manager；
- [ ] Pull 基于 Cursor 分页；
- [ ] Pull 失败时 Cursor 不前进；
- [ ] Remote Apply 不产生 Outbox；
- [ ] 冲突持久化且可解决；
- [ ] Tombstone 不会被离线设备复活；
- [ ] Snapshot 支持断点续传；
- [ ] 新 Profile 可从 Cloud 完整恢复；
- [ ] 应用重启后同步继续；
- [ ] Cloud 重启后同步继续；
- [ ] 多用户数据严格隔离；
- [ ] Windows 同步状态 UI 可用；
- [ ] 原 EPIC-02、EPIC-03、EPIC-04 测试继续通过；
- [ ] CI 全部通过；
- [ ] 仓库中不存在明文 Token、密码或测试凭据；
- [ ] Android 未被误报为已实现。

---

# 27. Git 工作流

开始：

```bash
git fetch origin
git switch main
git pull --ff-only origin main
git switch -c agent/epic-05-windows-sync-core
```

建议提交顺序：

```text
docs(epic05): add precondition and ownership design
feat(profile): add local profile ownership model
feat(sync): add SQLite sync schema and repositories
feat(sync): add Windows client sync engine
feat(sync): make local writes transactional with outbox
feat(sync): implement push and token refresh
feat(sync): implement pull tombstones and conflicts
feat(sync): implement resumable snapshot initialization
feat(desktop): add sync scheduling and status UI
test(sync): add multi-profile and cloud end-to-end tests
docs(epic05): add completion report
```

完成后：

```bash
git status
git diff --check
cargo fmt --check
```

推送：

```bash
git push -u origin agent/epic-05-windows-sync-core
```

PR 标题：

```text
feat(sync): implement EPIC-05 Windows client synchronization
```

不得自动合并。

---

# 28. Agent 最终报告

Agent 最终必须输出：

## 实施信息

```text
Branch
Head SHA
修改文件
新增 Migration
新增模块
```

## 用户归属

明确说明：

```text
本地业务表保存的 ID 是什么
Cloud User ID 保存在哪里
首次登录如何绑定
切换账号如何隔离
退出登录是否保留数据
```

## 同步验证

逐项报告：

```text
Outbox Transaction
Push
Pull
Token Refresh
Conflict
Tombstone
Snapshot
Restart Recovery
Multi-User Isolation
Windows Build
Frontend Build
Cloud E2E
CI Run
```

## 安全验证

必须回答：

```text
Access Token 保存位置
Refresh Token 保存位置
Token 是否进入 SQLite
Token 是否进入日志
客户端是否能指定任意 Cloud User ID
是否使用客户端时间解决冲突
```

## 未完成项

只列出真正未完成的内容和精确原因。

没有真实测试证据时，不得声明：

```text
EPIC-05 已完成
```
