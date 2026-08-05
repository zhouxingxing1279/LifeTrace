# EPIC-04 前置条件审计报告

> 首次审计日期：2026-08-05  
> 首次审计基线：`0f4aec466d1c9f6db0b1f62e2a819f63291d5b16`  
> 首次审计结论：**0 / 8 通过，EPIC-04 正式认证实现必须停止。**  
> PostgreSQL 修复基线：`0719bcaab3bb4ded26f95929b90c71dac972a304`  
> 当前复核结论：**8 / 8 通过，本实现合并后 EPIC-04 可进入阶段 1。**

## 1. 首次审计范围

首次审计严格执行 `LifeTrace_EPIC04_Agent_Implementation_Plan.md` 的阶段 0，只核验 EPIC-03 PostgreSQL 持久化门禁，不实现账号、密码、Token、Session、设备管理、登录 UI 或任何内存认证替代方案。

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

## 2. 首次门禁结果

| 门禁项 | 首次结果 | 首次代码证据 | 首次判定 |
|---|---|---|---|
| `lifetrace-cloud` 已引入 SQLx PostgreSQL | ❌ 未通过 | `Cargo.toml` 没有 `sqlx` 依赖；EPIC-03 完成报告写明运行接线待完成 | 当前服务无法创建或使用 SQLx PostgreSQL 连接池 |
| `AppState` 持有真实 `PgPool` | ❌ 未通过 | `AppState` 持有 `Arc<RwLock<Store>>`，没有 `PgPool` | 运行时仍是进程内状态 |
| 启动时实际执行 SQLx Migration | ❌ 未通过 | `main.rs` 只创建 `AppState` 并启动 Axum；迁移脚本是占位提示 | `MIGRATION_ON_STARTUP` 没有执行效果 |
| `/health/ready` 检查 PostgreSQL | ❌ 未通过 | Ready 只校验配置和开发 AuthProvider，并返回 `storage: memory` | Ready 不能证明数据库可用 |
| PostgreSQL 不可用时 Ready 返回 503 | ❌ 未通过 | Ready 状态码只取决于配置校验，没有数据库探测 | 数据库宕机时仍可能返回 200 |
| 服务重启后实体、Change Log、幂等状态仍存在 | ❌ 未通过 | `Store` 使用 `HashMap` 保存所有同步状态 | 进程退出后同步状态丢失 |
| Docker Compose 中 Cloud 实际连接 PostgreSQL | ❌ 未通过 | Compose 注入 `DATABASE_URL`，但运行时代码没有 SQLx/PgPool | 环境变量存在不等于实际连接数据库 |
| PostgreSQL 集成测试可运行 | ❌ 未通过 | HTTP 测试只创建内存应用，没有数据库断言 | 现有测试只验证内存协议状态机 |

**首次门禁总计：0 / 8 通过。**

## 3. 首次发现的阻塞项

1. 引入 SQLx PostgreSQL，并建立受配置约束的 `PgPool`。
2. 将 `AppState` 改为持有真实数据库连接池。
3. 在服务启动阶段连接 PostgreSQL，并执行内嵌 Migration。
4. 将 Entity、Processed Change、Change Log、Snapshot 和 Identity 数据迁移到 PostgreSQL Repository/Transaction。
5. 让 `/health/ready` 探测数据库，并在不可用时返回 503。
6. 让 Docker Compose 实际启动 Cloud、连接 PostgreSQL、执行 Migration 并通过重启验证。
7. 增加真实 PostgreSQL 集成测试。
8. 将迁移和测试脚本从占位实现改为可执行流程。
9. 更新 EPIC-03 完成报告。

## 4. 首次审计的停止决定

首次审计时依据硬性门禁：

- 未实现账号注册、登录、登出或密码管理；
- 未创建正式 Access Token、Refresh Token、Web Session 或 Password Reset Token；
- 未在内存 Store、SQLite、JSON 或前端状态中保存真实认证凭据；
- 未修改登录 UI；
- 未执行 EPIC-04 阶段 1 及后续阶段。

该停止决定在 PostgreSQL 门禁修复完成前保持有效。

## 5. PostgreSQL 修复复核

2026-08-05 对 `agent/epic-03-postgres-runtime` 重新执行门禁审计。

| 门禁项 | 复核结果 | 修复证据 |
|---|---|---|
| `lifetrace-cloud` 已引入 SQLx PostgreSQL | ✅ 通过 | `Cargo.toml` 已启用 SQLx PostgreSQL、Migration、Chrono、UUID 和 JSON；解析后的 `Cargo.lock` 已提交 |
| `AppState` 持有真实 `PgPool` | ✅ 通过 | `AppState.pool` 为 `PgPool`；配置数据库时使用 `PostgresRepository` |
| 启动时实际执行 SQLx Migration | ✅ 通过 | `AppState::initialize()` 先探测数据库，再运行 `sqlx::migrate!()`；失败会阻止监听 |
| `/health/ready` 检查 PostgreSQL | ✅ 通过 | Ready 在超时内执行 `SELECT 1`，并返回 PostgreSQL 检查状态 |
| PostgreSQL 不可用时 Ready 返回 503 | ✅ 通过 | PostgreSQL 集成测试使用不可连接地址验证 503 |
| 服务重启后实体、Change Log、幂等状态仍存在 | ✅ 通过 | 重建 `AppState` 后 Entity、Pull、Duplicate 结果和 Change Log 数量保持正确 |
| Docker Compose 中 Cloud 实际连接 PostgreSQL | ✅ 通过 | CI 实际启动 PostgreSQL 16 与 Cloud，Ready 通过；重启 Cloud 后 Ready 再次通过 |
| PostgreSQL 集成测试可运行 | ✅ 通过 | 空库 Migration、持久化、幂等、Pull、Snapshot、Ready 503 全部通过 |

**复核门禁总计：8 / 8 通过。**

详细证据见：

```text
docs/epic-03/postgres-validation.md
docs/epic-03/completion-report.md
.github/workflows/epic03-postgres.yml
```

关键 GitHub Actions 流水线：

```text
30974868575  全步骤通过
30975150235  全步骤通过并固定 Cargo.lock
```

## 6. 当前处理决定

本实现合并到 `main` 后：

- EPIC-03 PostgreSQL 前置阻塞解除；
- EPIC-04 可以进入阶段 1：Auth Contract；
- 正式密码、Token、Session、App Grant、Device 和 Audit 数据必须使用 PostgreSQL；
- 禁止重新引入内存认证 Store；
- Windows 本地模式仍不得以登录作为启动和读取本地数据的前置条件。

本报告只解除 EPIC-04 的 PostgreSQL门禁，不代表已经实现 EPIC-04 的账号与认证功能。
