# ADR-005：协议 / schema / 实体 / 客户端版本分离

- 状态：已采纳（EPIC-02）

## 背景

协议演进、实体结构演进、客户端发布互不耦合，混为一谈会导致频繁的破坏性升级。

## 决策

明确区分：

| 概念 | 作用 |
|---|---|
| `protocolVersion` | 线路协议（Push/Pull/Snapshot）版本 |
| `schemaVersion` | 整体领域契约版本 |
| `entitySchemaVersion` | 单个实体 payload 版本 |
| `clientVersion` | App 发布版本 |
| `appId` | 客户端产品标识 |

v1 规则：

- 允许新增可选字段、新增 entity type、新增可选元数据（不升 protocolVersion）。
- 破坏性修改（重命名/删字段、改含义、改 cursor/幂等/冲突语义）必须升 protocolVersion。
- 未知 JSON 字段可忽略；未知枚举值以字符串保留。
- 客户端过旧返回 HTTP 426 + `LIFETRACE_CLIENT_TOO_OLD`。

## 后果

- 每个 entity type 在注册表中独立记录 `schemaVersion`。
- 破坏性变更需版本协商；`capabilities` 暴露 supported/minimum 版本。
