# EPIC-03 PostgreSQL 前置门禁验证

> 验证日期：2026-08-05  
> 实现分支：`agent/epic-03-postgres-runtime`  
> 实现基线：`0719bcaab3bb4ded26f95929b90c71dac972a304`  
> CI：`EPIC-03 PostgreSQL`

## 结论

EPIC-04 阶段 0 定义的 8 项 EPIC-03 PostgreSQL 前置门禁全部通过。

| 门禁项 | 结果 | 实现与验证证据 |
|---|---|---|
| `lifetrace-cloud` 已引入 SQLx PostgreSQL | ✅ | `services/lifetrace-cloud/Cargo.toml` 启用 SQLx PostgreSQL、runtime、migration、chrono、uuid 和 json；生成后的 `Cargo.lock` 已提交 |
| `AppState` 持有真实 `PgPool` | ✅ | `src/state.rs` 的 `AppState.pool: PgPool`；有数据库配置时使用 `PostgresRepository` |
| 启动时实际执行 SQLx Migration | ✅ | `AppState::initialize()` 先执行 `SELECT 1`，再根据 `MIGRATION_ON_STARTUP` 执行 `sqlx::migrate!()`；失败会阻止监听端口 |
| `/health/ready` 检查 PostgreSQL | ✅ | Ready 在 2 秒超时内执行 `SELECT 1`，响应包含 `storage=postgresql` 和 `postgresql=true/false` |
| PostgreSQL 不可用时 Ready 返回 503 | ✅ | `readiness_fails_when_postgres_is_unavailable` 使用不可连接地址验证 503 |
| 服务重启后 Entity、Change Log、幂等状态仍存在 | ✅ | PostgreSQL 集成测试销毁并重建 `AppState` 后验证 Entity、Pull 和 Duplicate 结果；Change Log 数量保持 1 |
| Docker Compose 中 Cloud 实际连接 PostgreSQL | ✅ | 工作流实际构建并启动 PostgreSQL 16 与 Cloud，检查 Ready；重启 Cloud 后再次检查成功 |
| PostgreSQL 集成测试可运行 | ✅ | 空库 Migration、持久化、幂等、Pull、Snapshot、Ready 503 全部在 GitHub Actions PostgreSQL 16 服务中通过 |

**门禁总计：8 / 8 通过。**

## 持久化边界

生产运行路径通过 `SyncRepository` 使用 `PostgresRepository`。持久化范围包括：

```text
cloud_users
cloud_devices
sync_entities
sync_processed_changes
sync_change_log
sync_snapshots
sync_snapshot_items
```

内存 Repository 仅用于不调用运行时配置校验的进程内协议测试。Cloud 可执行程序缺少 `DATABASE_URL` 时拒绝启动，不能静默回退到内存存储。

## 事务边界

Push 的一次请求复用同一数据库连接和外层事务。单个 Change 在事务中完成：

```text
幂等查询
→ Entity Registry 与 Payload 校验
→ Dependency 校验
→ SELECT Entity FOR UPDATE
→ Base Version 校验
→ Change Log 写入并取得 Cursor
→ Entity/Tombstone 更新
→ Processed Change 写入
```

Atomic Group 使用嵌套事务 Savepoint；组内任一结果失败时回滚该组的数据库写入。

## Migration 验证

- 保留 `0001` 至 `0006`，未修改历史 Migration；
- 新增 `0007_runtime_persistence.sql`；
- 空数据库可执行全部 Migration；
- `_sqlx_migrations` 记录不少于 7 项；
- 重复启动不会重复破坏数据；
- Migration 或数据库连接失败时服务不能开始监听；
- `lifetrace-migrate` 可作为独立 Migration Job 使用。

## CI 验证

工作流依次执行：

```text
cargo fmt
contract tests
cloud tests
production clippy -D warnings
docker build
docker compose up --wait
ready assertion
cloud restart
ready assertion again
```

验证流水线：

- `30974868575`：全部步骤通过；
- `30975150235`：全部步骤通过，并提交解析后的 Cloud `Cargo.lock`。

## EPIC-04 决策

本分支合并后，EPIC-04 不再受 PostgreSQL 前置条件阻塞，可以进入阶段 1。正式认证数据必须继续使用本次建立的 PostgreSQL 运行路径，禁止重新引入内存密码、Token 或 Session Store。
