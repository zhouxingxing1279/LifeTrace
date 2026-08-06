# EPIC-05 完成报告

## 实施范围

EPIC-05 实现 Windows/Tauri 本地优先同步闭环，不包含 Android。新增纯 Rust `lifetrace-sync-client`、SQLite Profile/Outbox/State/Conflict/Snapshot 适配、HTTP Transport、后台调度、Tauri 命令、前端同步状态和跨平台 CI。

## 数据库变更

- Migration 0007：稳定 Local Profile、Active Profile、历史 Owner 迁移、Outbox、State、Conflict、Metadata、Snapshot Staging；
- Migration 0008：业务表本地写入 Trigger Outbox，以及 Remote/Migration 抑制上下文；
- Repository 写入统一使用 Active LocalProfileId，读取按 Profile 隔离。

## 同步能力

- 离线可写，业务与 Outbox 同事务；
- Push 幂等、Lease、部分成功、Retry、429、413 自适应拆批和原子组保护；
- Cursor Pull 分页，页应用失败不推进 Cursor；
- Remote Apply 不产生 Outbox；
- Conflict 持久化和三种解决方式；
- Tombstone 与服务端版本 Metadata；
- Snapshot Staging、Resume 和 Cursor 过期恢复；
- 本地变更、周期和 Retry 到期调度；
- 应用重启后依赖数据库状态继续。

## 用户与安全

- 本地业务 Owner 为 LocalProfileId；CloudUserId 只用于绑定；
- Cloud User 来自 `/auth/me` 或 Refresh 响应，绑定 API 不接受客户端 Cloud User ID；
- Access Token 仅在进程内存；
- Refresh Token 仅 Windows Credential Manager，不写 SQLite；
- 同步日志不输出 Token；
- Cloud 端继续按认证 Principal 强制用户隔离；
- 冲突比较使用 Server Version，不以客户端时钟决定覆盖。

## 验证

正式验证由 `.github/workflows/epic05-windows-sync.yml` 执行：

1. Linux：Rustfmt、Contract、Pure Core、Tauri SQLite、Clippy、Frontend；
2. PostgreSQL：Cloud 认证、持久化与多用户隔离回归；
3. Windows：Pure Core、Tauri Library 和 Frontend Build。

实施分支的正式提交由验证工作流在全部门禁成功后产生，因此仓库中出现该提交即表示基础发布门禁已通过。完整场景与证据映射见 `test-matrix.md`。

## 明确未实现

Android、Kotlin SDK、Room、WorkManager、Android Credential Storage 和 Android UI 均未实现，也未在本报告中声明完成。
