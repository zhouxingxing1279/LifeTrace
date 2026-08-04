# ADR-003：服务端 cursor 与版本

- 状态：已采纳（EPIC-02）

## 背景

客户端 `updatedAt` 与本地时钟不可作为全局顺序或冲突依据（时钟偏移、离线设备）。

## 决策

- 服务端为每个 change 分配单调递增的 `cursor`（wire 为 opaque string）。
- 服务端为每个实体维护权威 `serverVersion`；客户端提交时携带已知的 `baseServerVersion`。
- `clientModifiedAt` 仅用于审计、展示与时钟偏移诊断。

## 理由

- Pull 必须无缝隙、无重复、严格升序，只有服务端顺序能保证。
- 冲突判断需要权威版本号，不能依赖客户端时间。
- cursor/serverVersion 使用字符串避免 JavaScript 安全整数问题。

## 后果

- 客户端不得加减/猜测 cursor，不得按 `updatedAt` 重排。
- cursor 过期（超出保留期）返回 `LIFETRACE_CURSOR_EXPIRED`，客户端先做 Snapshot。
