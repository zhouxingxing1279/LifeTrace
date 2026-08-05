# EPIC-04 前置条件审计报告

> 审计日期：2026-08-05  
> 审计仓库：`zhouxingxing1279/LifeTrace`  
> 审计分支：`main`  
> 审计基线：`0f4aec466d1c9f6db0b1f62e2a819f63291d5b16`  
> 结论：**未通过 EPIC-03 前置门禁，EPIC-04 正式认证实现必须停止。**

## 1. 审计范围

本轮严格执行 `LifeTrace_EPIC04_Agent_Implementation_Plan.md` 的阶段 0，只核验 EPIC-03 PostgreSQL 持久化门禁，不实现账号、密码、Token、Session、设备管理、登录 UI 或任何内存认证替代方案。

核验范围：

- `services/lifetrace-cloud/Cargo.toml`
- `services/lifetrace-cloud/src/state.rs`
- `services/lifetrace-cloud/src/main.rs`
- `services/lifetrace-cloud/src/config.rs`
- `services/lifetrace-cloud/src/store.rs`
- `services/lifetrace-cloud/src/routes/health.rs`
- `services/lifetrace-cloud/src/auth/provider.rs`
- `services/lifetrace-cloud/tests/api.rs`
- `services/lifetrace-cloud/migrations/0001-0006`
- `deploy/cloud/docker-compose.local.yml`
- `deploy/cloud/docker-compose.test.yml`
- `scripts/cloud/migrate.ps1`
- `scripts/cloud/test.ps1`
- `docs/epic-03/completion-report.md`

## 2. 前置门禁结果

| 门禁项 | 结果 | 代码证据 | 判定 |
|---|---|---|---|
| `lifetrace-cloud` 已引入 SQLx PostgreSQL | ❌ 未通过 | `Cargo.toml` 没有 `sqlx` 依赖；EPIC-03 完成报告也明确写明“PostgreSQL + SQLx 运行接线待完成” | 当前服务无法创建或使用 SQLx PostgreSQL 连接池 |
| `AppState` 持有真实 `PgPool` | ❌ 未通过 | `AppState` 持有 `Arc<RwLock<Store>>`，没有 `PgPool` | 运行时仍是进程内状态 |
| 启动时实际执行 SQLx Migration | ❌ 未通过 | `main.rs` 只创建 `AppState` 并启动 Axum；没有连接数据库或执行 migration；`migrate.ps1` 仍是占位提示 | `MIGRATION_ON_STARTUP=true` 目前只被解析，没有执行效果 |
| `/health/ready` 检查 PostgreSQL | ❌ 未通过 | Ready 只校验配置和开发 AuthProvider，并返回 `storage: memory` | Ready 不能证明数据库可用 |
| PostgreSQL 不可用时 Ready 返回 503 | ❌ 未通过 | Ready 的状态码只取决于 `config.validate()`；没有数据库探测 | 数据库宕机时仍可能返回 200 |
| 服务重启后实体、Change Log、幂等状态仍存在 | ❌ 未通过 | `Store` 使用 `HashMap` 保存 users、entities、processed changes、change log 和 snapshots；文件注释明确说明 persistence 不在当前原型范围 | 进程退出后全部同步状态丢失 |
| Docker Compose 中 Cloud 实际连接 PostgreSQL | ❌ 未通过 | Compose 注入了 `DATABASE_URL`，但运行时代码没有 SQLx/PgPool；测试 Compose 仅启动 PostgreSQL，没有 Cloud 服务 | 环境变量存在不等于应用实际连接数据库 |
| PostgreSQL 集成测试可运行 | ❌ 未通过 | `tests/api.rs` 通过 `AppState::new(Config::default())` 创建内存应用；没有数据库 fixture、migration、重启持久化或 PostgreSQL 查询断言 | 现有 HTTP 测试只验证内存协议状态机 |

**门禁总计：0 / 8 通过。**

## 3. 已存在但尚未接线的资产

以下内容可在修复 EPIC-03 时复用，但不能视为门禁已通过：

- PostgreSQL migration SQL：`0001_cloud_identity.sql` 至 `0006_indexes.sql`；
- `Config` 中的 `DATABASE_URL`、连接数和 `MIGRATION_ON_STARTUP` 字段；
- 本地与测试 PostgreSQL Compose 配置；
- 面向替换的 `AuthProvider` 边界；
- 内存状态机及其 HTTP 协议测试。

## 4. EPIC-03 阻塞项

必须先完成以下工作：

1. 在 `lifetrace-cloud` 引入 SQLx PostgreSQL，并建立受配置约束的 `PgPool`。
2. 将 `AppState` 改为持有真实数据库连接池；生产和默认 Cloud 路径不得回退到内存 Store。
3. 在服务启动阶段连接 PostgreSQL，并在 `MIGRATION_ON_STARTUP=true` 时执行内嵌 migration；失败时阻止服务进入 Ready。
4. 将同步实体、processed changes、change log、snapshot 和 identity 数据读写迁移到 PostgreSQL repository/transaction。
5. 让 `/health/ready` 执行有超时的数据库探测；数据库不可用时返回 503。
6. 修复并验证 Docker Compose：Cloud 容器实际启动、连接 PostgreSQL、运行 migration，并能通过持久化卷跨服务重启保留数据。
7. 增加真实 PostgreSQL 集成测试：空库 migration、Push/Pull、幂等、冲突、snapshot、用户隔离、服务重建后的持久化。
8. 将 `scripts/cloud/migrate.ps1` 从占位脚本改为可执行 migration job，并让测试脚本对 PostgreSQL 行为做断言，而不只是同时启动一个未被使用的数据库。
9. 更新 EPIC-03 completion report，使“完成”状态与实际运行接线一致。

## 5. 对 EPIC-04 的处理决定

依据 EPIC-04 计划中的硬性门禁，本轮：

- 不实现账号注册、登录、登出或密码管理；
- 不创建正式 Access Token、Refresh Token、Web Session 或 Password Reset Token；
- 不在 `Store`、SQLite、JSON 或前端状态中保存真实密码、Token 或 Session；
- 不修改登录 UI；
- 不执行 EPIC-04 阶段 1 及后续阶段；
- 不声称 EPIC-04 已开始正式开发。

下一步应先完成 EPIC-03 PostgreSQL 运行接线并通过 8 项门禁，再重新执行本报告中的审计。

## 6. 本轮验证

本报告完成后执行了静态一致性检查，确认：

- 8 个门禁项均有明确结果和代码证据；
- 所有失败项均对应至少一个修复动作；
- 未加入任何密码、Token、Cookie Secret 或其他敏感值；
- 未修改 EPIC-03 migration；
- 未创建或修改正式认证代码；
- 报告结论符合 EPIC-04 阶段 0 的停止条件。
