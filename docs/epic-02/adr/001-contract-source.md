# ADR-001：公共契约权威来源为 Rust contract crate

- 状态：已采纳（EPIC-02）

## 背景

仓库同时存在 TypeScript 前端类型（`src/types/*`）、Rust Row 结构（`src-tauri/src/database/repositories`）与散落的 `serde_json` DTO，三处重复且彼此漂移。

## 决策

新建 `crates/lifetrace-contracts`，以 **Rust 类型作为公共领域与 Wire DTO 的权威来源**，并以此为唯一输入生成 JSON Schema、TypeScript 与 OpenAPI。

## 理由

- 桌面与未来云后端都是 Rust，Rust 类型可直接共享。
- `serde` + `schemars` + `ts-rs` 可稳定生成三种下游产物。
- 单一来源避免三份定义漂移。
- 契约 crate 不依赖 Tauri / Axum / rusqlite / React，可被任何端复用。

## 后果

- 修改公共类型只能改 Rust；前端 Wire DTO 不再手工定义。
- 生成物（`contracts/`）禁止手工编辑，`npm run contracts:check` 校验过期。
- Database Row、Domain Model、Wire DTO、UI View Model 四层分离。
