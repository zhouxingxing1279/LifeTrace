# EPIC-03 API 实现说明

## 端点

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/health/live` | 否 | 进程存活 |
| GET | `/health/ready` | 否 | 就绪检查 |
| GET | `/api/v1/meta/version` | 否 | 服务与协议版本 |
| GET | `/api/v1/sync/capabilities` | 否 | 能力与版本协商 |
| POST | `/api/v1/sync/push` | 是 | 批量提交（幂等 + 原子组） |
| POST | `/api/v1/sync/pull` | 是 | 按签名 cursor 拉取 |
| POST | `/api/v1/sync/snapshot` | 是 | 全量快照（一致视图 + 分页） |
| GET/POST/DELETE | `/api/v1/finance/transactions...` | 是 | 业务 CRUD 示例 |

## Push

- Create：实体不存在且 `baseServerVersion=0` → `serverVersion=1`。
- Update：`baseServerVersion == 当前版本` → `serverVersion+1`。
- Conflict：版本不匹配 → 返回当前实体/tombstone + reason，不修改服务器。
- 幂等：`userId + changeId`；同 changeId 同 hash → `duplicate`；不同 hash → `CHANGE_ID_REUSE`。
- Atomic Group：同组全成或全败；组内按 `entityType + entityId` 固定顺序（内存实现已排序）。

## Pull

- 严格按 cursor 升序；所有查询带 `user_id`。
- `nextCursor` 为签名 token，绑定用户与 entity scope。
- cursor 过期 → `410 LIFETRACE_CURSOR_EXPIRED`，要求 Snapshot。

## Snapshot

- 创建时捕获一致视图（内存实现复制 items；PostgreSQL 用 REPEATABLE READ）。
- 分页使用签名 page token（绑定 user + snapshot）。
- 完成后客户端把 cursor 设为 `snapshotCursor`，继续 Pull 无缝。

## 状态

四端点已在内存存储上实现并通过 16 个 HTTP 集成测试；PostgreSQL 版本复用同一业务层。
