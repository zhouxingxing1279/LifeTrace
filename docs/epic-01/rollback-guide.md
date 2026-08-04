# EPIC-01 回滚指南

## 原则

- Migration 全部在事务中执行，失败自动 ROLLBACK，`schema_migrations` 不写成功记录
- 迁移前自动备份到 `%APPDATA%\com.lifetrace.desktop\backups\database\`
- 旧 JSON 表重命名为 `legacy_*_json_v1` 保留，可用于回溯恢复

## 迁移失败时

1. 应用会拒绝启动并输出 `数据库迁移失败: Migration vN 失败: ...`
2. 检查 `migration_runs` 最后一条状态为 `failed`（含备份路径）
3. 失败事务已回滚，数据库结构保持迁移前状态；无需手工处理
4. 修复代码或数据后重新启动即可重试

## 需要人工回滚时

1. 停止应用
2. 从 `backups\database\` 选择最近一次成功的 `lifetrace-before-schema-v*.db`
3. 先复制当前库留档（例如 `lifetrace.db.before-rollback-<时间戳>`）
4. 用备份文件替换 `%APPDATA%\com.lifetrace.desktop\lifetrace.db`
5. 如存在旧 `-wal` 文件，一并移走（避免 WAL 重放旧数据）
6. 启动应用确认数据完整

## 数据回溯

即使新表已生效，旧数据仍在 `legacy_*_json_v1` 表中（只读）。需要恢复单条记录时，
可从 `migration_issues.raw_json` 或 legacy 表取回原始 JSON。

## 备份验证

每次备份创建后都会打开并执行 `PRAGMA integrity_check`，不通过则视为备份失败并中止迁移。
