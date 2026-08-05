# EPIC-03 PostgreSQL 运行接线完成报告

> 日期：2026-08-05  
> 实现分支：`agent/epic-03-postgres-runtime`  
> 验证工作流：`EPIC-03 PostgreSQL`

## 完成结论

EPIC-03 原有同步协议和内存状态机已经保留，本次补齐了此前缺失的 PostgreSQL 生产运行路径。EPIC-04 要求的 8 项 PostgreSQL 前置门禁现已全部满足。

独立 Cloud 可执行程序不再允许无数据库启动；内存 Repository 仅保留给进程内协议测试使用，不属于可部署运行路径。

## 本次补完内容

1. 在 `lifetrace-cloud` 引入 SQLx PostgreSQL，并固定生成后的 `Cargo.lock`。
2. `AppState` 持有真实 `PgPool`，配置了连接数和连接获取超时。
3. Cloud 启动前执行数据库连接探测，并在 `MIGRATION_ON_STARTUP=true` 时运行内嵌 SQLx Migration。
4. 新增独立迁移命令 `lifetrace-migrate`，替换原占位迁移脚本。
5. 新增异步 `SyncRepository` 边界和 `PostgresRepository` 实现。
6. 将以下状态持久化到 PostgreSQL：
   - Cloud User 和 Device 安装锚点；
   - Sync Entity；
   - Processed Change 幂等结果；
   - Change Log 和 Tombstone；
   - Snapshot 与 Snapshot Item。
7. Push 在单个 PostgreSQL 事务中完成版本校验、实体锁、状态更新、Change Log 和幂等结果写入；Atomic Group 使用数据库 Savepoint 回滚。
8. Pull 按 `(user_id, cursor)` 严格读取，保留 Entity Scope、签名 Cursor、分页和 Tombstone 行为。
9. Snapshot 数据和分页状态可跨进程状态重建继续读取。
10. `/health/ready` 使用有超时的 `SELECT 1` 检查 PostgreSQL；数据库不可用时返回 503。
11. Finance CRUD 示例改为通过统一 Repository 持久化，不再直接访问进程内 Store。
12. 修复 Docker 构建上下文、非 root 运行、容器 Healthcheck 和 Compose Cloud/PostgreSQL 接线。
13. Cloud 配置在缺少 `DATABASE_URL`、连接池边界非法或生产启用 DEV_AUTH 时 Fail Closed。

## Migration

保留已发布的 `0001` 至 `0006`，只追加：

```text
0007_runtime_persistence.sql
```

该 Migration 补充稳定的外部设备标识和 Change/Entity 的来源设备标识，不修改既有 Migration。

## 验证结果

GitHub Actions 使用 PostgreSQL 16 和 Rust 1.88 运行：

- Contract 测试：61 项通过；
- Cloud 单元测试：7 项通过；
- 既有 HTTP/API 集成测试：16 项通过；
- PostgreSQL 集成测试：2 项通过；
- Rust 测试合计：86 项通过；
- 生产 Library/Binary Clippy：`-D warnings` 通过；
- Docker Runtime 镜像构建通过；
- Compose 实际启动 PostgreSQL 与 Cloud 通过；
- Ready 响应确认 `storage=postgresql` 且数据库检查为 true；
- 重启 Cloud 容器后 Ready 再次通过；
- 生成后的 Cloud `Cargo.lock` 已提交。

关键验证流水线：

```text
30974868575  完整 PR 验证，全步骤通过
30975150235  分支验证、Compose 重验和 Cargo.lock 固定，全步骤通过
```

PostgreSQL 测试明确覆盖：

- 空库执行全部 Migration；
- Push 后重建 `AppState`，Entity 仍存在；
- Processed Change 在重建后仍能返回 Duplicate；
- Change Log 不重复增长；
- Pull 可读取持久化 Change；
- Snapshot 可读取持久化 Entity；
- PostgreSQL 不可用时 Ready 返回 503。

## EPIC-04 前置状态

EPIC-04 阶段 0 的 PostgreSQL 门禁已由 `docs/epic-03/postgres-validation.md` 和 `docs/epic-04/precondition-report.md` 复核为 **8 / 8 通过**。

EPIC-04 可以在本实现合并后进入阶段 1：Auth Contract。正式账号、密码、Token、Session、App Grant 和设备撤销仍属于 EPIC-04，不在本次提交中提前实现。

## 后续非阻塞工作

以下内容不再阻塞 EPIC-04，但可在后续迭代继续加固：

- 大规模数据的 `EXPLAIN ANALYZE` 和性能基准；
- 更完整的并发压力与故障注入测试；
- Notes、English、Habits 等业务 CRUD 路由；
- 生产可观测性和运维告警；
- EPIC-04 引入正式数据库认证实现后移除 Development Auth 的部署使用。
