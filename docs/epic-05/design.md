# EPIC-05 Windows 同步设计

## 总体架构

```text
业务 Repository / 本地 Axum API
        │ SQLite 事务
        ▼
业务表 + Trigger Outbox
        │ Notify / 30 秒维护 / 5 分钟周期
        ▼
SyncDesktopState Scheduler
        │ 全局 Async Run Gate
        ▼
纯 Rust SyncEngine
  Push → Pull → Snapshot Recovery
        │
        ▼
HttpSyncTransport + EPIC-04 Credential Provider
        │
        ▼
LifeTrace Cloud PostgreSQL
```

## 模块边界

- `crates/lifetrace-sync-client`：平台无关状态机、Push/Pull/Snapshot、Retry、错误分类和测试工具，不依赖 Tauri/SQLite。
- `src-tauri/src/sync/store.rs`：`SyncStore` 的 SQLite 适配器，负责 Lease、Cursor、冲突、Remote Apply 与 Snapshot Staging。
- `src-tauri/src/sync/transport.rs`：HTTP 适配器、401 Single-Flight Refresh、429/5xx/413 分类。
- `src-tauri/src/sync/runtime.rs`：Profile、认证会话、调度、手动同步和 UI 状态。
- `src-tauri/src/sync/commands.rs`：最小 Tauri 命令边界。
- `src/services/cloudSync.ts` 与 `CloudAccountPanel.tsx`：前端桥与状态 UI。

## 本地数据库

`sync_outbox` 保存 `change_id`、实体引用、操作、基础版本、Payload、依赖、原子组、状态、Lease、Retry 和错误信息。`sync_state` 按 Profile/Scope 保存 Cursor、阶段、统计和重试时间。`sync_conflicts` 保存本地与远端 Payload。`sync_metadata` 保存实体服务端版本和 Tombstone 状态。`sync_snapshot_staging` 支持分页断点和事务性切换。

## Push

1. 仅租用当前 Active Profile 的到期 Pending 记录；
2. 若首批包含 `atomic_group_id`，同一原子组全部进入同一 Lease；
3. 服务端部分成功逐项 Confirm/Conflict/Blocked；
4. Offline、Transient、429 按持久化 Retry 时间释放 Lease；
5. 413 对非原子批次递归拆分；不可拆原子组进入 Dead Letter；
6. `change_id` 保证幂等。

## Pull 与冲突

Pull 按 Cursor 分页。每页在一个 SQLite 事务中应用，成功后才推进 Cursor。存在本地 Pending 变更时不静默覆盖，而是写入 `sync_conflicts`。远端 Tombstone 更新 Metadata 并删除本地实体；未解决的本地旧写入不能复活已删除远端实体。

## Snapshot

新 Cloud Profile、强制重新初始化或 Cursor 过期时进入 Snapshot。每页先写 Staging 并记录 Resume Token；全部完成后在事务内替换当前 Profile 的远端副本并设置 Snapshot Cursor。失败保留进度，重启后继续。

## 调度

调度器支持：本地写入唤醒后 2 秒防抖、30 秒维护 Tick、5 分钟周期同步、持久化 `next_retry_at` 到期触发。`SyncDesktopState.run_gate` 与 `SyncEngine.run_lock` 保证同进程只执行一个同步周期。

## 离线原则

未登录、断网、认证失效和 Cloud 不可用都不阻断本地 CRUD。同步失败只更新状态和重试时间，不回滚已经提交的本地业务事务。
