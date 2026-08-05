# EPIC-03 前置审计报告

> 日期：2026-08-05

## 1. 前置依赖确认

- EPIC-01 数据层重构：已完成（6 个 migration，`docs/epic-01/`）。
- EPIC-02 统一领域模型与同步协议：已提交并可编译测试（`crates/lifetrace-contracts`，61+2 个测试通过）。
- `EntityType` / `SyncChangeV1` / Push / Pull / Snapshot / Tombstone / Conflict / ErrorCode：均已定义。
- Entity Registry：所有可同步实体已登记，`secret_local_only` 不进入注册表。
- 金额整数分、localVersion/serverVersion 分离、未知字段与未知枚举向前兼容：均已满足。

## 2. 同步实体 ID

`EntityId` 为字符串 newtype，兼容历史非 UUID ID；关系实体（`note.tag_relation`、`note.relation` 等）都有独立同步 ID，不依赖复合主键。

## 3. 当前云端代码状态

审计前仓库不存在云端服务；本报告发布时已随阶段 1-7 一并落地：

- `services/lifetrace-cloud`（Rust + Axum）
- 四个同步端点 + 服务端状态机（内存存储，按用户隔离）
- AuthProvider 边界（Development / Test）
- Canonical JSON + Change Hash + 签名 Cursor / Page Token

## 4. 阻塞项

- PostgreSQL + SQLx 运行依赖无法在本机离线拉取，PostgreSQL 实现在本报告发布时尚未接线（migration SQL 已就绪）。
- 正式认证、Token、设备注册属于 EPIC-04，本 Epic 只做边界。

## 5. 需要调整的内容

- 同步引擎保持可替换存储接口，PostgreSQL 接入时公开方法不变。
- Cursor 已从明文数字改为 HMAC 签名 opaque token。
