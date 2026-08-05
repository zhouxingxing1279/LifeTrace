# EPIC-03 数据库 Schema

见 `services/lifetrace-cloud/migrations/`，共 6 个 migration：

| Migration | 内容 |
| --- | --- |
| 0001_cloud_identity | `cloud_users`、`cloud_devices`（身份锚点，无凭据） |
| 0002_sync_entities | 通用同步实体权威副本（JSONB + payload_hash + tombstone 约束） |
| 0003_processed_changes | `userId + changeId` 幂等记录 |
| 0004_change_log | 服务端变更日志（cursor BIGSERIAL，operation 约束） |
| 0005_snapshots | `sync_snapshots` + `sync_snapshot_items`（keyset 分页主键） |
| 0006_indexes | 用户/类型/cursor 关键索引 |

要点：

- `entity_id` 使用 TEXT（兼容历史非 UUID ID）。
- JSONB 不是无约束 JSON：写入前必须经 `EntityPayload` 类型化校验。
- `secret_local_only` 类型永不进入同步表。
- Snapshot 分页禁止 OFFSET，使用 `(entity_type, entity_id)` keyset。

> 状态：migration SQL 已就绪；SQLx 接线待恢复联网拉取依赖后完成。
