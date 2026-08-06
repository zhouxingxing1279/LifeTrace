# EPIC-05 同步状态机

## 状态

- `local_only`：Profile 未绑定 Cloud，完整本地可用；
- `idle`：已绑定但尚未运行；
- `initializing_snapshot`：Snapshot 初始化或 Cursor 恢复；
- `pushing`：租用并上传 Outbox；
- `pulling`：按 Cursor 下载远端变化；
- `up_to_date`：本轮成功且无未解决冲突；
- `retry_wait`：可重试错误，等待 `next_retry_at`；
- `auth_required`：Refresh Token 缺失、过期或被撤销；
- `conflict`：存在未解决冲突；
- `blocked`：永久错误或需要用户处理；
- `upgrade_required`：协议/Schema 不兼容。

## 主要转移

```text
local_only --绑定--> idle
idle --新档案/强制--> initializing_snapshot
idle/retry_wait --触发--> pushing
pushing --成功--> pulling
pulling --成功且无冲突--> up_to_date
pulling --冲突--> conflict
pulling --Cursor 过期--> initializing_snapshot
任意网络阶段 --Offline/429/5xx--> retry_wait
任意网络阶段 --401 且刷新失败--> auth_required
auth_required --重新登录--> idle
conflict --全部解决--> idle/up_to_date
```

## Cursor 原子性

Pull 页先在事务中完成实体、Metadata、Conflict 和 Tombstone 更新，最后写 Cursor。任意一步失败则事务回滚，Cursor 保持原值。

## Lease 与重启恢复

Outbox Lease 包含 Owner 与 Expiry。崩溃后到期 Lease 可重新租用。Retry Count、Next Attempt、错误码和错误消息均持久化。Snapshot Resume Token 与 Staging 同样持久化，所以应用重启后可继续。

## 错误分类

- `Offline`、`Transient`：指数退避；
- `RateLimited`：优先使用 `Retry-After`；
- `PayloadTooLarge`：保持原子组边界的自适应拆批；
- `AuthRequired`：Single-Flight Refresh，失败后停止云同步；
- `PermissionDenied`、`Permanent`：Blocked/Dead Letter；
- `UpgradeRequired`：显式升级状态；
- Cursor 过期类错误：Snapshot 恢复后再次 Pull。
