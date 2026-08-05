# lifetrace-cloud

LifeTrace EPIC-03 独立云端后端服务（Rust + Axum）。

实现依据：[docs/LifeTrace_EPIC03_Agent_Implementation_Plan.md](../docs/LifeTrace_EPIC03_Agent_Implementation_Plan.md)

## 端点

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/health/live` | 否 | 存活 |
| GET | `/health/ready` | 否 | 就绪 |
| GET | `/api/v1/meta/version` | 否 | 版本 |
| GET | `/api/v1/sync/capabilities` | 否 | 能力协商 |
| POST | `/api/v1/sync/push` | Bearer | 批量提交 |
| POST | `/api/v1/sync/pull` | Bearer | 拉取变更 |
| POST | `/api/v1/sync/snapshot` | Bearer | 全量快照 |
| GET/POST/DELETE | `/api/v1/finance/transactions...` | Bearer | CRUD 示例 |

## 认证（EPIC-03 边界）

```text
Authorization: Bearer <DEV_AUTH_TOKEN>
```

默认 `dev-token`。生产环境强制关闭 DEV_AUTH、强制签名密钥（`Config::validate`）。

## 运行

```powershell
cargo run --manifest-path services/lifetrace-cloud/Cargo.toml
```

环境变量见 `deploy/cloud/.env.example`。当前存储为内存（每用户隔离）；
PostgreSQL migration SQL 在 `migrations/`，接入 SQLx 后启用 `DATABASE_URL`。

## 关键实现

- 幂等：Canonical JSON → Change Hash（`sync/payload_hash.rs`）
- Cursor / Page Token：HMAC-SHA256 签名，绑定 user + scope（`sync/cursor_codec.rs`、`sync/page_token.rs`）
- 状态机：`store.rs`（实体 / change log / tombstone / 原子组 / snapshot 一致视图）

## 测试

```powershell
cargo test --manifest-path services/lifetrace-cloud/Cargo.toml
```

20 个测试（16 API 集成 + 4 单元）覆盖协议语义与认证边界。

## 部署

见 `deploy/cloud/`（compose / Dockerfile / Caddy 示例）与 `docs/epic-03/`。
