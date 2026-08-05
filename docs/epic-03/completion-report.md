# EPIC-03 完成报告

> 日期：2026-08-05

## 交付内容

1. 独立云端服务 `services/lifetrace-cloud`（Rust + Axum）
2. 同步协议 v1 四端点（Capabilities / Push / Pull / Snapshot）
3. 服务端状态机：实体、change log、cursor、tombstone、幂等、冲突、原子组、快照一致视图
4. Canonical JSON + Change Hash + 签名 Cursor / Page Token
5. AuthProvider 边界（Development / Test；生产禁用规则）
6. 健康检查（live / ready）、meta 版本、请求 ID、优雅关闭
7. 业务 CRUD 示例（finance）
8. PostgreSQL Migration SQL（0001-0006）+ Docker/Compose/Caddy 部署示例
9. 文档：本目录 9 篇 + crate README

## 测试

- 16 个 HTTP 集成测试 + 4 个单元测试全部通过
- 覆盖：health、auth（无 token / 错 token / 正常）、create/update/delete、冲突、
  幂等、changeId 重用、未知实体、协议版本、原子组、分页顺序、快照一致性 +
  后续 Pull 无缝、cursor 过期、用户隔离、CRUD 互通、cursor 编解码、canonical JSON

## 已知问题与下一步

- PostgreSQL + SQLx 运行接线待恢复联网后完成（migration 已就绪；存储接口保持可替换）
- 正式认证 / 设备注册属 EPIC-04
- 其余业务 CRUD（notes/english/habits/reviews/xunji/files/timeline/reports）复用 finance 模式

## EPIC-04 接口点

- 替换 `AuthProvider`（token 校验 → `AuthenticatedPrincipal`）
- `cloud_users` / `cloud_devices` 由注册/登录流程写入

## EPIC-05 接口点

- 使用四个同步端点 + 签名 cursor + snapshotCursor 续拉；冲突按 `keep_server` / `keep_local` / `manual_merge` 处理
