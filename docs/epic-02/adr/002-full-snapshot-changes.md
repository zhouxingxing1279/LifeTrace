# ADR-002：Sync Change v1 使用完整实体快照

- 状态：已采纳（EPIC-02）

## 背景

变更可以表达为字段级 JSON Patch，也可以表达为完整实体快照。

## 决策

`SyncChangeV1.operation=upsert` 的 payload 必须是**完整实体快照**；v1 不使用 JSON Patch。

## 理由

- 首版实现更稳定、更易重试与校验。
- 不依赖补丁顺序，降低 schema 演进风险。
- 跨 Rust / TypeScript / Kotlin 表达更简单。
- 后续可在新 protocol 版本中增加字段级合并。

## 后果

- payload 体积更大；由 `maximumRequestBytes` 与分页/批处理限制管理。
- delete 默认不携带 payload，服务端生成 tombstone。
