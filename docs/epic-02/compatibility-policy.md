# LifeTrace EPIC-02 版本兼容策略

## 1. 五个版本概念

| 概念 | 含义 | 例子 |
|---|---|---|
| `protocolVersion` | Push/Pull/Snapshot 线路协议版本 | `1` |
| `schemaVersion` | 整体领域契约版本 | `1` |
| `entitySchemaVersion` | 单个实体 payload 版本 | `1`（每个 entity type 独立） |
| `clientVersion` | App 发布版本 | `0.2.1` |
| `appId` | 客户端产品标识 | `lifetrace-desktop` |

## 2. v1 允许不升 protocolVersion 的变化

- 新增可选字段
- 新增客户端可忽略的响应元数据
- 新增 entity type（需同步更新实体注册表与 `supportedEntityTypes`）
- 新增错误详情字段
- 放宽限制

## 3. 必须升级 protocolVersion 的变化

- 重命名或删除必填字段
- 改变字段含义
- 改变 Push/Pull 状态机
- 改变 cursor 语义
- 改变幂等规则
- 删除既有状态
- 改变冲突行为

## 4. 向前兼容规则

- **未知 JSON 字段必须可忽略**（serde 默认行为，有测试覆盖）。
- **未知枚举值不得导致整批解析失败**：v1 的 wire 枚举全部实现为字符串 newtype（`Unknown(String)` 语义），例如 `ChangeOperation`、`TransactionType`、`ErrorCode` 等。
- 客户端太旧：HTTP `426 Upgrade Required` + `LIFETRACE_CLIENT_TOO_OLD`，响应包含 `minimumClientVersion` / `minimumProtocolVersion` / `minimumSchemaVersion`。

## 5. 变更流程

1. 修改 `crates/lifetrace-contracts` 中的 Rust 类型（唯一权威来源）。
2. 运行 `npm run contracts:generate` 重新生成 JSON Schema / OpenAPI / TypeScript。
3. 运行 `npm run contracts:test`（含 golden fixture 兼容性校验）与 `npm run contracts:check`。
4. 若破坏既有 v1 兼容性：升级 `protocolVersion` / `schemaVersion` 并更新本文件与 OpenAPI。

## 6. Golden Fixture

`crates/lifetrace-contracts/tests/fixtures/sync-v1/*.json` 是冻结的 v1 兼容样本；发布后修改类型必须保证旧 fixture 仍可解析，否则视为破坏性变更。
