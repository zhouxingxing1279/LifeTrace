# lifetrace-sync-server

LifeTrace EPIC-03 独立云端后端服务（Axum 原型）。

实现依据：

- `docs/LifeTrace_Complete_Roadmap_v3.md` 中 EPIC-03 章节
- `lifetrace-contracts` 同步协议 v1（四个端点 + 服务端状态机）

## 端点

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/healthz` | 健康检查 |
| GET | `/api/v1/sync/capabilities` | 能力与版本协商 |
| POST | `/api/v1/sync/push` | 批量提交变更（幂等 + 原子组） |
| POST | `/api/v1/sync/pull` | 按服务端 cursor 拉取变更 |
| POST | `/api/v1/sync/snapshot` | 全量快照（一致视图 + 分页） |
| POST | `/api/v1/devices/register` | 设备注册占位（EPIC-04 接管） |
| GET/POST | `/api/v1/finance/transactions` | 财务交易 CRUD 示例 |
| GET/DELETE | `/api/v1/finance/transactions/{id}` | 财务交易查询/删除 |

## 运行

```powershell
cargo run --manifest-path crates/lifetrace-sync-server/Cargo.toml
```

默认监听 `127.0.0.1:8787`。可用环境变量：

```text
LIFETRACE_SERVER_BIND=127.0.0.1:8787
LIFETRACE_MAX_PUSH_BATCH_SIZE=500
LIFETRACE_MAX_PULL_BATCH_SIZE=200
LIFETRACE_MAX_REQUEST_BYTES=4194304
LIFETRACE_MAX_SNAPSHOT_PAGE_SIZE=200
LIFETRACE_MAX_ATOMIC_GROUP_SIZE=50
LIFETRACE_RETENTION_ENTRIES=1000
```

## 身份（占位）

当前通过请求头 `X-LifeTrace-User` 区分用户（默认 `dev-user`），存储层按用户严格隔离。
正式认证、Token 与设备注册属于 EPIC-04。

## 存储

当前为内存存储（每用户独立状态机）：实体表、change log、cursor、tombstone、幂等记录、
快照一致视图、原子组全部按协议语义实现。持久化采用可替换的存储层设计，后续接入
PostgreSQL + SQLx 时保持公开方法不变。

## 测试

```powershell
cargo test --offline --manifest-path crates/lifetrace-sync-server/Cargo.toml
```

15 个 HTTP 集成测试覆盖：创建/更新/删除、base version 冲突、重复 changeId 幂等、
changeId 重用拒绝、未知实体类型、协议版本 426、原子组全成/全败、Pull 分页顺序、
Snapshot 一致性 + 后续 Pull 无缝、cursor 过期要求 Snapshot、用户隔离、设备注册占位、
财务 CRUD 与同步链路互通。

## 尚未实现（后续迭代）

- PostgreSQL 持久化（需要 sqlx/postgres 依赖，当前沙箱离线无法拉取）
- 正式认证 / Token（EPIC-04）
- 业务 CRUD 模块：finance 已完成示例；notes、english、habits、reviews、xunji、files、timeline、reports 待实现
- 云端部署与运维（EPIC-19）
