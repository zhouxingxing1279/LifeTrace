# EPIC-05 前置审计报告

## 审计范围

本次审计覆盖 Windows/Tauri 本地 SQLite、所有业务 Repository、本地 Axum 写接口、EPIC-02 同步契约、EPIC-03 Cloud PostgreSQL 同步端点以及 EPIC-04 登录、刷新令牌和设备身份实现。Android、Kotlin、Room 与 WorkManager 不在本阶段范围内。

## 现有能力

- Cloud 已提供 `/api/v1/sync/capabilities`、`push`、`pull`、`snapshot` 端点，并使用契约 crate 中的 v1 DTO。
- Cloud 认证可签发和刷新 Access Token；Refresh Token 具备轮换与撤销能力。
- Windows 客户端已有 SQLite Migration Runner、按领域划分的 Repository、Tauri 命令桥和 Windows Credential Manager 凭据封装。
- Finance、Habits、Daily Review、Notes、English、Workout 已存在本地 CRUD 路径。

## 发现的缺口

1. 历史业务表大量使用固定 `user_id='local'`，无法表达稳定档案和多账号隔离。
2. 本地写入没有统一 Outbox，离线变更无法可靠重放。
3. 缺少 Cursor、Conflict、Tombstone、Snapshot Staging、Lease 与 Retry 持久化状态。
4. 登录后没有“绑定当前档案”与“新建云端档案”的显式选择。
5. 同步入口缺少全局串行化、变更触发、定时触发和重启恢复。
6. 前端只展示登录状态，缺少待上传数量、冲突数量和手动同步入口。

## 用户归属问题

必须区分：

- `LocalProfileId`：SQLite 业务数据归属，稳定 UUID；
- `CloudUserId`：从服务器验证后的 Access Token Principal 获得；
- `AppId`：固定应用身份，例如 `lifetrace-desktop`；
- `DeviceId`：当前安装实例身份。

客户端请求参数不得决定 Cloud 数据所有者。服务端必须始终以认证 Principal 的 `user_id` 为准。

## 需要迁移的本地数据

Migration 0007 创建 `local_profiles`、`active_profile`、`sync_outbox`、`sync_state`、`sync_conflicts`、`sync_metadata`、`sync_snapshot_staging` 和同步上下文表，并将历史占位所有者迁移到默认 UUID Profile。迁移覆盖 Finance、Activities/Habits、Daily Reviews、Notes、English 与 Workout 相关业务表。

Migration 0008 为可同步业务表创建 INSERT、UPDATE、DELETE Trigger。Trigger 与业务 SQL 处于同一 SQLite 事务；Remote/Migration Apply 通过同步上下文抑制 Outbox。

## 写入入口清单

- Finance：账户、分类、交易；
- Habits：活动、活动日志；
- Reviews：每日复盘；
- Notes：笔记、文件夹、标签；
- English：文章、学习记录、词汇；
- Workouts：训练、动作、组记录等 Registry 支持实体；
- 本地 Axum 成功的 POST/PUT/PATCH/DELETE：发送同步唤醒信号。

## 协议结论

现有 v1 Contract 足以承载 Push、Pull、Snapshot、Conflict、Tombstone、Cursor 和设备信息。客户端补充 413 自适应拆批、原子组不可拆分保护、Cursor 过期触发 Snapshot 恢复，以及统一错误分类。

## 主要风险与控制

- **占位 owner 残留**：Migration 测试与 Repository owner 覆盖测试。
- **Outbox 与业务写入不原子**：SQLite Trigger，同一事务提交。
- **远端应用形成回声**：Remote Apply 上下文抑制 Trigger。
- **并发 Worker 重复执行**：进程级 Run Gate、Engine Run Lock 和数据库 Lease。
- **大原子组超过服务端限制**：不拆原子组，转 Dead Letter 并保留诊断。
- **客户端伪造 Cloud User ID**：`/auth/me` 验证身份，绑定命令不接受 Cloud User ID 参数。
- **凭据泄漏**：Refresh Token 仅 Windows Credential Manager；Access Token 仅内存。
