# LifeTrace EPIC-02 校验报告

> 日期：2026-08-05  
> 范围：EPIC-02「统一领域模型与同步协议」  
> 依据：`docs/epic-02/implementation-plan.md`（Definition of Done，第 33 节）  
> 结论：**代码与 Rust 侧验证全部通过；前端验证待在有 Node/npm 的环境执行后，方可正式宣告完成。**

---

## 1. 交付物清单

| 交付物 | 位置 | 状态 |
| --- | --- | --- |
| 当前契约审计 | `docs/epic-02/current-contract-audit.md` | 已完成 |
| 公共契约 crate | `crates/lifetrace-contracts/` | 已完成 |
| 基础值对象 | `src/ids.rs`、`src/time.rs`、`src/money.rs`、`src/common.rs` | 已完成 |
| 领域 DTO | `src/domain/*.rs`（财务、习惯、复盘、笔记、英语、训记、文件、关联、偏好、用户、设备） | 已完成 |
| 实体注册表 | `src/registry.rs` | 已完成 |
| 同步协议 v1 DTO | `src/sync/v1/*.rs`（Push/Pull/Snapshot/Tombstone/Conflict/Capabilities） | 已完成 |
| 稳定错误码 | `src/error.rs` | 已完成 |
| 内存参考实现 | `src/sync/testkit.rs` | 已完成 |
| Golden Fixture | `tests/fixtures/sync-v1/*.json`（9 个） | 已完成 |
| JSON Schema | `contracts/json-schema/*.schema.json`（95 个） | 已完成 |
| TypeScript 生成物 | `contracts/typescript/lifetrace-contracts.generated.ts` | 已完成 |
| OpenAPI | `contracts/openapi/lifetrace-sync-v1.json` | 已完成 |
| 契约生成器 | `tools/contract-exporter/` | 已完成 |
| 桌面端契约适配 | `src-tauri/src/contracts.rs`（财务交易端到端示例） | 已完成 |
| 文档 | `docs/epic-02/` 下 7 篇文档 + 5 篇 ADR | 已完成 |

---

## 2. Definition of Done 逐项核对

- [x] 已生成当前契约审计 —— `current-contract-audit.md`
- [x] 有独立 `lifetrace-contracts` crate —— `crates/lifetrace-contracts`
- [x] contract crate 不依赖 Tauri、Axum、rusqlite —— 依赖仅 chrono / schemars / serde / serde_json / thiserror / ts-rs
- [x] 财务、习惯、复盘、笔记、英语、训记领域 DTO 已定义 —— `src/domain/`
- [x] 文件元数据与跨实体关联 DTO 已定义 —— `domain/files.rs`、`domain/links.rs`
- [x] 用户和设备 DTO 已定义 —— `domain/user.rs`
- [x] 所有 entity type 有稳定注册表 —— `registry.rs`（唯一名称、ownership/sync/conflict 全覆盖测试）
- [x] 同步协议 v1 已定义 —— `sync/v1/`
- [x] Push 幂等语义明确 —— 重复 changeId、changeId 重用不同 payload 均有测试
- [x] Pull 使用服务端 cursor —— opaque string newtype，按 cursor 升序测试通过
- [x] Delete 使用 tombstone —— tombstone 结构 + 双方删除幂等测试
- [x] Snapshot 协议已定义 —— snapshot 一致性 + snapshot 后 Pull 无缝测试
- [x] 冲突不依赖客户端时间 —— baseServerVersion 冲突测试，禁默认 LWW
- [x] `version` 与 `serverVersion` 职责分离 —— `EntityMeta.local_version` 与 `server_version` 分离
- [x] protocol/schema/client/app 版本职责明确 —— 兼容策略文档 + 测试
- [x] 未知字段可兼容 —— `unknown_fields_are_ignored` 测试
- [x] 未知枚举不会导致整批同步崩溃 —— `unknown_enum_values_do_not_fail_batch_parsing` 测试
- [x] 有稳定错误码 —— 必需错误码齐全且不可变
- [x] 有 Golden Fixture —— 9 个 fixture，解析测试全部通过
- [x] 有 JSON Schema —— 95 个 schema 文件
- [x] 有 TypeScript 生成物 —— `lifetrace-contracts.generated.ts`
- [x] 有 OpenAPI —— `lifetrace-sync-v1.json`
- [x] 有内存参考实现 —— `sync/testkit.rs`
- [x] 重复 changeId 测试通过
- [x] changeId 不同 payload 重用测试通过
- [x] 冲突测试通过
- [x] tombstone 测试通过
- [x] cursor 分页测试通过
- [x] snapshot 测试通过
- [x] atomic group 测试通过
- [x] 桌面端至少一个实体完成契约适配 —— `contracts.rs`，2 个测试通过
- [x] 未提前实现生产同步 Worker —— 无 outbox / 后台 Worker / 网络同步代码
- [ ] 前端测试通过 —— **待执行**（沙箱无 Node/npm 访问权限）
- [x] Rust 测试通过 —— 61 + 2 个测试全部通过（见下）
- [ ] 构建通过 —— Rust 侧编译通过；前端构建**待执行**
- [x] 文档完整 —— 7 篇文档 + 5 篇 ADR

---

## 3. 实测验证结果

### 3.1 契约 crate 测试

```text
cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml
```

结果：**61 个测试全部通过，0 失败**

- 单元测试 41 个：ID/金额/时间/错误码/注册表/领域 DTO/枚举 round-trip
- Golden fixture 9 个：push-request、push-success、push-conflict、pull-response、delete-tombstone、snapshot-page、capabilities、error-response
- Round-trip 11 个：请求/响应序列化、未知字段忽略、未知枚举批解析

### 3.2 桌面端契约适配测试

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib contracts
```

结果：**2 个测试全部通过**

- 历史非 UUID ID 保留
- Transaction DB row → Contract DTO → SyncChangeV1 JSON round-trip 字段一致

### 3.3 生成物一致性

```text
cargo run --manifest-path tools/contract-exporter/Cargo.toml
git diff --exit-code -- contracts/
```

结果：**无 diff**，生成物与 Rust 权威来源一致（等价 `npm run contracts:check` 的 Rust 侧验证）。

### 3.4 依赖隔离

`crates/lifetrace-contracts/Cargo.toml` 仅声明：

```toml
chrono, schemars, serde, serde_json, thiserror, ts-rs
```

无 Tauri / Axum / rusqlite / React 依赖，符合硬性边界。

### 3.5 提交记录

EPIC-02 按阶段拆分为 9 个提交（`bc70a60..82be20e`），已推送至远程 `origin/main`：

```text
docs(contracts): audit current domain and api models
feat(contracts): add shared contract crate
feat(contracts): define public domain DTOs
feat(contracts): add entity type registry
feat(sync): define sync protocol v1 wire DTOs
test(sync): add in-memory protocol testkit
feat(contracts): generate json schema openapi and typescript
feat(desktop): add contract adapters
docs(sync): document protocol v1 and compatibility policy
```

---

## 4. 待完成项

以下命令需要在本机（有 Node/npm 的环境）执行，沙箱对 `nodejs-portable` 目录无访问权限：

```powershell
npm.cmd run lint          # tsc --noEmit
npm.cmd test              # lint + test:unit + web:build
npm.cmd run test:rust     # cargo test src-tauri
npm.cmd run build         # tauri build
npm.cmd run contracts:check
```

全部通过后，EPIC-02 可正式宣告完成。

---

## 5. 未解决问题

- 前端验证依赖 Node 环境，当前沙箱无法执行（非代码问题）。
- `docs/LifeTrace_Complete_Roadmap_v3.md`、EPIC-01/EPIC-02 实施计划为未跟踪文档，随本次校验一并纳入版本管理。
