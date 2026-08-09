# LifeTrace Sync Protocol v1

> 端点前缀 `/api/v1/sync`。本协议只定义契约（DTO + OpenAPI + 参考实现），生产云端服务属于 EPIC-03，客户端同步引擎属于 EPIC-05。
> 参考实现：`crates/lifetrace-contracts/src/sync/testkit.rs`（仅测试用，禁止用于生产）。
> 权威 OpenAPI：`contracts/openapi/lifetrace-sync-v1.json`。

---

## 1. 端点

| 方法 | 路径 | 请求 | 响应 |
|---|---|---|---|
| GET | `/api/v1/sync/capabilities` | - | `CapabilitiesResponseV1` |
| POST | `/api/v1/sync/push` | `PushRequestV1` | `PushResponseV1` |
| POST | `/api/v1/sync/pull` | `PullRequestV1` | `PullResponseV1` |
| POST | `/api/v1/sync/snapshot` | `SnapshotRequestV1` | `SnapshotResponseV1` |

## 2. SyncChangeV1

```json
{
  "changeId": "a1b2c3d4-...",
  "entityType": "finance.transaction",
  "entityId": "tx-1",
  "operation": "upsert",
  "baseServerVersion": "0",
  "entitySchemaVersion": 1,
  "clientModifiedAt": "2026-08-04T15:30:00Z",
  "payload": { "...": "完整实体快照" },
  "atomicGroupId": null,
  "dependencies": []
}
```

- `operation` 仅 `upsert` / `delete`；未知操作按字符串保留（不解析失败）。
- upsert payload 是**完整实体快照**，不是 JSON Patch；payload 内 `meta.id` 必须等于 `entityId`，payload 类型必须匹配 `entityType`。
- delete 默认不带 payload；服务端生成 tombstone。
- `clientModifiedAt` 只用于审计/展示/时钟偏移诊断，**不得**用于全局顺序或冲突自动解决。

## 3. Push

`PushRequestV1 { requestId, client, changes[] }`

- 幂等键：`userId + changeId`。
  - 相同 changeId + 相同 payload → 返回首次结果（`duplicate`），不重复写入。
  - 相同 changeId + 不同 payload → `LIFETRACE_CHANGE_ID_REUSE`。
  - `requestId` 仅用于追踪，不替代 changeId。
- 每个 Change 独立返回 `accepted` / `duplicate` / `conflict` / `rejected`；业务冲突不使整个 HTTP 请求失败。

### Atomic Group

- 相同 `atomicGroupId` 的 changes 必须在同一请求中。
- 全部成功或全部失败；任一项冲突/拒绝 → 整组 `LIFETRACE_ATOMIC_GROUP_FAILED`，组内不产生任何写入。
- 组大小上限由 `capabilities.maximumAtomicGroupSize` 声明（v1 默认 50）。

## 4. Pull

`PullRequestV1 { requestId, client, afterCursor?, limit, entityTypes? }`

- 响应按服务端 cursor **严格升序**；客户端按序应用，**不得**按 `updatedAt` 重排。
- 整批成功后才保存 `nextCursor`；中途失败不得提前推进。
- `afterCursor` 是 opaque string，客户端不能加减或猜测。
- cursor 已过期（超出 tombstone 保留期/change log 保留窗口）→ `LIFETRACE_CURSOR_EXPIRED`，客户端必须先做 Snapshot。

`ServerChangeV1` 字段：`cursor, entityType, entityId, operation, serverVersion, serverModifiedAt, payload?, tombstone?, originDeviceId?`。

## 5. Tombstone

```json
{
  "entityType": "finance.transaction",
  "entityId": "tx-1",
  "deletedAt": "2026-08-04T15:31:00Z",
  "serverVersion": "12",
  "deletedByDevice": "22222222-..."
}
```

- delete 必须进入 change log（不能直接消失）。
- 被删除实体重新创建必须使用**新 ID**。
- 恢复软删除必须显式基于 tombstone 的最新 `serverVersion` 提交。
- 双方删除：版本一致视为幂等成功（返回当前 tombstone 版本）；版本不一致返回 `conflict`（`reason=both_deleted`）。

## 6. Snapshot

`SnapshotRequestV1 { requestId, client, snapshotId?, pageToken?, entityTypes?, pageSize }`

`SnapshotResponseV1 { requestId, snapshotId, snapshotCursor, items[], nextPageToken?, completed, serverTime }`

- 首次请求不带 `snapshotId`，服务端分配；后续页必须携带同一 `snapshotId` + `pageToken`。
- 同一 snapshot 所有页面对应**一致视图**（分页期间并发修改不泄漏进已捕获视图）。
- 客户端完成后：把 cursor 设为 `snapshotCursor`，再从该 cursor 开始 Pull——并发变化无缝隙。
- `completed=true` 后不再有 `nextPageToken`。

## 7. Capabilities

`GET /api/v1/sync/capabilities` 至少返回：

`protocolVersion`、`supportedProtocolVersions`、`schemaVersion`、`minimumSchemaVersion`、`minimumClientVersions`、`maximumPushBatchSize`、`maximumPullBatchSize`、`maximumRequestBytes`、`maximumSnapshotPageSize`、`maximumAtomicGroupSize`、`tombstoneRetentionDays`、`supportedEntityTypes`、`serverTime`。

v1 默认：push 500 / pull 200 / request 4 MiB / snapshot page 200 / atomic group 50 / tombstone 90 天。

## 8. 冲突

- **禁止默认 Last-Write-Wins**。
- 当 `client baseServerVersion != current serverVersion`（含实体已删除/已恢复等状态变化）→ 返回 `conflict`，附服务端当前实体或 tombstone。
- 解决方式由客户端实现：
  - `keep_server`：丢弃本地待同步变更，应用服务端实体。
  - `keep_local`：以服务端最新版本为新的 `baseServerVersion`，**生成新的 changeId** 重新 Push，保留冲突审计。
  - `manual_merge`：合并后生成新 changeId、基于最新版本 Push。

## 9. HTTP 状态码

| 状态码 | 场景 |
|---|---|
| 200 | 请求成功处理（含部分/全部 change 的 accepted/conflict/rejected） |
| 400 | 请求结构错误、schema 不支持 |
| 401 / 403 | 未认证 / 设备或用户无权限 |
| 410 | cursor 过期 |
| 413 | 请求体过大 |
| 426 | 客户端或协议版本过旧（`LIFETRACE_CLIENT_TOO_OLD`） |
| 429 | 限流 |
| 500 | 服务内部错误 |
| 503 | 暂时不可用 |

## 10. 请求头（建议）

`X-LifeTrace-App-Id`、`X-LifeTrace-Client-Version`、`X-LifeTrace-Protocol-Version`、`X-LifeTrace-Schema-Version`、`X-LifeTrace-Device-Id`、`X-Request-Id`；请求体仍携带 `client`，服务端可校验一致。

## 11. 参考实现测试

`crates/lifetrace-contracts` 测试覆盖：Create、Update、Base version conflict、重复 changeId、changeId 不同 payload 重用、Delete tombstone、双方删除、Pull cursor 顺序、Pull 分页、Snapshot 一致性、Snapshot 后 Pull、Atomic group、未知 entity type、过期 cursor、不支持的协议版本。
