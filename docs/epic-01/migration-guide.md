# EPIC-01 Migration 指南

## 运行机制

启动顺序（`src-tauri/src/server.rs`）：

```text
打开 lifetrace.db（database::connection::open）
→ PRAGMA WAL / foreign_keys=ON / busy_timeout=5000
→ database::migration_runner::run()（版本化 Migration，含自动备份）
→ 模块 ensure_schema / 种子数据（只保留配置类表）
→ 旧 D1 导入（仅未导入时，经 database::legacy::d1_import）
→ 启动 HTTP 服务 127.0.0.1:3103
```

每个 Migration 执行前自动用 SQLite Backup API 创建一致性备份到
`%APPDATA%\com.lifetrace.desktop\backups\database\lifetrace-before-schema-vN-*.db`，
备份后执行 `PRAGMA integrity_check` 并计算 SHA-256；保留最近 3 份。

## Migration 版本

| 版本 | 名称 | checksum | 内容 |
|---|---|---|---|
| 1 | framework | m0001-framework-v1 | Migration/备份框架落表，app_meta 收归版本化 |
| 2 | finance-normalization | m0002-finance-v1 | 财务：账户/分类/交易/证据真实列 + amount_cents |
| 3 | habits-reviews-normalization | m0003-habits-reviews-v1 | 习惯/打卡/复盘真实列 + 唯一约束 |
| 4 | notes-normalization | m0004-notes-v1 | 笔记元数据真实列 + 标签/关系/附件/版本 + FTS5 |
| 5 | english-normalization | m0005-english-v1 | 英语文章/记录/高亮/笔记/生词/分析 + occurrences/review_state |
| 6 | workouts-imports-normalization | m0006-workouts-v1 | 训练/动作/组/导入/训练笔记真实列 |

已应用的版本记录在 `schema_migrations(version, name, checksum, applied_at, app_version)`；
运行记录在 `migration_runs`；异常在 `migration_issues`。

## 数据流

- 读取：真实列 → Repository → 旧 camelCase DTO → 前端
- 写入：旧 DTO → 校验/转换（金额 cents）→ Repository → 新表
- 金额：数据库内部永远 `amount_cents`；返回前端 `/100.0`
- 核心表不再写回 `data_json`（`settings` 等 KV 表除外）

## 旧表保留

迁移时旧 JSON 表重命名为 `legacy_*_json_v1` 并保留数据（只读回溯），不删除、不覆盖。
旧库文件本身从未被修改。

## 从旧版本升级

1. 备份 `%APPDATA%\com.lifetrace.desktop\lifetrace.db`（含 -wal/-shm）
2. 启动新版应用 → 自动执行 Migration（每步先自动备份）
3. 首次启动后检查 `schema_migrations` 已到版本 6，`migration_runs` 全部 succeeded
4. 如失败：见 rollback-guide.md
