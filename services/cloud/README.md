# LifeTrace Cloud

LifeTrace 独立云端后端服务（Rust + Axum + PostgreSQL）。它与桌面应用位于同一个 Monorepo，但可以独立构建、测试和部署。

## 目录

- 云端源码：`services/cloud/src/`
- 数据库迁移：`services/cloud/migrations/`
- 云端测试：`services/cloud/tests/`
- 部署文件：`deploy/cloud/`
- 共享协议：`crates/lifetrace-contracts/`、`contracts/`

## 端点

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/health/live` | 否 | 存活 |
| GET | `/health/ready` | 否 | 就绪 |
| GET | `/api/v1/meta/version` | 否 | 版本 |
| GET | `/api/v1/sync/capabilities` | 否 | 能力协商 |
| POST | `/api/v1/sync/push` | Bearer / Web Session | 批量提交 |
| POST | `/api/v1/sync/pull` | Bearer / Web Session | 拉取变更 |
| POST | `/api/v1/sync/snapshot` | Bearer / Web Session | 全量快照 |

## 运行

从仓库根目录：

```powershell
npm run dev:cloud
```

或直接：

```powershell
cargo run --manifest-path services/cloud/Cargo.toml
```

本地 PostgreSQL / Docker 环境见 `deploy/cloud/` 和 `scripts/cloud/`。

## 测试

```powershell
npm run test:cloud
```

或：

```powershell
cargo test --manifest-path services/cloud/Cargo.toml
```

## 部署

生产镜像使用 `services/cloud/Dockerfile`。云端服务不依赖 `apps/desktop`，部署时只需要云端源码、共享 Rust crates 与部署配置。
