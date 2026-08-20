# Web 财务单一数据源架构

## 目标

LifeTrace Web 财务与 BeeCount Android/兼容客户端必须读取和写入同一套权威数据，不再为 Web 额外依赖独立 BeeCount Cloud 数据库。

## 最终数据流

```text
BeeCount Android
  -> BeeCount compatibility API
  -> LifeTrace PostgreSQL
     -> sync_entities / sync_change_log

LifeTrace Web
  -> /api/v1/integrations/beecount/*
  -> BeeCountSyncService
  -> LifeTrace PostgreSQL
     -> sync_entities / sync_change_log
```

`sync_entities` 与 `sync_change_log` 是财务实体的权威存储。`beecount_identity_links`、`beecount_entity_clocks` 等 `beecount_*` 表只保存 BeeCount 协议兼容所需的身份、时钟、共享和迁移元数据，不形成第二份交易事实表。

## Web API

以下 Web 财务接口由 `services/cloud/src/routes/beecount_web.rs` 提供：

- `GET /api/v1/integrations/beecount/status`
- `GET /api/v1/integrations/beecount/ledgers`
- `GET /api/v1/integrations/beecount/ledgers/{ledger_id}/snapshot`

这些接口直接复用 `BeeCountSyncService`，不调用外部 BeeCount Cloud，也不需要 `BEECOUNT_ADAPTER_ENABLED=true` 才能工作。响应保留 BeeCount 兼容来源标识，同时返回 `storage: lifetrace-postgresql`，明确最终权威存储。

## 兼容边界

- BeeCount Android 继续使用 BeeCount wire-compatible API，客户端无需因为 Web 财务读取方式变化而修改。
- LifeTrace Web 继续使用原有 `/api/v1/integrations/beecount/*` URL，不改变前端路由。
- 数据库启用的真实 Cloud 部署不回退到历史 LifeTrace finance CRUD。
- `BEECOUNT_ADAPTER_*` 外部适配器代码目前仅作为历史兼容实现保留，不再位于 Web 财务运行时读取链路。

## 回归测试

`services/cloud/tests/beecount_web_single_source_postgres.rs` 会在外部 BeeCount adapter 保持关闭的情况下：

1. 通过 BeeCount compatibility API 注册并写入账本、账户、分类、标签、预算和交易；
2. 验证数据落在同一 PostgreSQL `sync_entities`；
3. 使用 Web 财务 API 读取相同账本与交易；
4. 验证 Web 状态返回 `enabled=true` 且 `storage=lifetrace-postgresql`。

该测试用于防止未来再次引入“BeeCount Android 一套库、LifeTrace Web 另一套库”的双数据源回归。
