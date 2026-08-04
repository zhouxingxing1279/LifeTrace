# LifeTrace EPIC-02 集成指南

## 1. 目录结构

```text
crates/lifetrace-contracts/   公共契约 crate（Rust 权威来源）
  src/ids.rs                   ID / Cursor / ServerVersion
  src/time.rs                  时间与自然日
  src/money.rs                 金额
  src/common.rs                EntityMeta
  src/error.rs                 错误码与 ApiErrorV1
  src/registry.rs              实体注册表
  src/domain/                  领域 DTO
  src/sync/v1/                 Sync v1 DTO
  src/sync/testkit.rs          内存参考实现（仅测试）
  tests/round_trip.rs          往返/兼容测试
  tests/golden.rs              Golden fixture 测试
  tests/fixtures/sync-v1/      冻结 v1 样本
tools/contract-exporter/       JSON Schema / OpenAPI / TypeScript 生成器
contracts/                     生成物（禁止手工编辑）
docs/epic-02/                  文档与 ADR
```

## 2. 常用命令

```powershell
npm.cmd run contracts:generate   # 重新生成 contracts/
npm.cmd run contracts:test       # 契约 crate 全部测试
npm.cmd run contracts:check      # 生成并校验 contracts/ 无过期 diff
cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml
```

## 3. 桌面端使用契约

`src-tauri/Cargo.toml` 已加入：

```toml
lifetrace-contracts = { path = "../crates/lifetrace-contracts" }
```

适配示例 `src-tauri/src/contracts.rs`：

```text
SQLite/Domain TransactionRow
  → transaction_row_to_contract()   → contracts::domain::Transaction
  → transaction_to_change()         → SyncChangeV1 (upsert)
  → serde_json                       → JSON
  → serde_json::from_value           → 反序列化
  → 字段一致性断言（测试）
```

要点：

- 金额以 `amountCents` 整数分传递，wire 上不存在 `amount`。
- `version`（本地修订）映射为 `meta.localVersion`；`serverVersion` 保持 `None`，新实体 `baseServerVersion="0"`。
- 历史非 UUID ID 原样保留。

## 4. 前端使用生成的 TypeScript

生成文件 `contracts/typescript/lifetrace-contracts.generated.ts` 已纳入 `tsconfig.json`（`npm run lint` 校验）。可在 `src/types/contracts.ts` 中 re-export：

```ts
export type { Transaction as ContractTransaction, SyncChangeV1, ... } from "../../contracts/typescript/lifetrace-contracts.generated";
```

UI View Model 仍可保持独立类型；公共 Wire DTO 不应再手工重复定义。

## 5. Android / Kotlin

本 Epic 不创建 Android App。基于 `contracts/openapi/lifetrace-sync-v1.json` 可用 OpenAPI Generator（`kotlin` / `kotlinx.serialization`）生成客户端 DTO；建议命令：

```powershell
openapi-generator-cli generate -i contracts/openapi/lifetrace-sync-v1.json -g kotlin -o build/android-client
```

## 6. 变更契约的步骤

1. 只修改 `crates/lifetrace-contracts` 的 Rust 类型。
2. `npm run contracts:generate`。
3. `npm run contracts:test`（golden fixture 兼容校验）。
4. `npm run contracts:check`（确认生成物一致）。
5. `npm run lint`（确认 TS 编译）。

## 7. EPIC-03 云端服务需要实现的接口

见仓库级最终报告；核心为四个端点 + 服务端状态机（实体存储、change log、cursor、tombstone、幂等、原子组、快照一致性视图、capabilities、认证/设备注册占位）。

## 8. EPIC-05 客户端同步需要实现的接口

见仓库级最终报告；核心为 SQLite outbox（同事务写入）、Push/Pull/Snapshot 调度、cursor 持久化、冲突解决（keep_server / keep_local / manual_merge）、后台 Worker。

## 9. 边界提醒

- 本 Epic 不实现：PostgreSQL 云端服务、正式认证、正式设备注册、正式 sync outbox、后台网络 Worker、自动重试调度、同步 UI、对象存储、Android App。
- `sync::testkit::SyncServer` 仅用于协议语义测试，禁止作为生产服务。
