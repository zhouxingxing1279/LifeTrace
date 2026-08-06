# EPIC-05 测试矩阵

| 领域 | 场景 | 自动化验证 |
|---|---|---|
| Profile Migration | 历史 `local` 替换为 UUID；同步表创建 | Migration 0007 单元测试 |
| Profile Isolation | 两个 Profile Repository 读取隔离；重复 Cloud 绑定拒绝 | `database::profile` 测试 |
| Owner Enforcement | 客户端 Payload 的 `userId` 被 Active Profile 覆盖 | `database::profile` 测试 |
| Outbox Transaction | Local 写入产生 Outbox；Remote Apply 被抑制 | Migration 0008 测试 |
| Contract Mapping | Finance 等 Legacy Payload 与 Wire Contract 映射 | `sync::payload` 测试 |
| Retry | 指数退避、上限和抖动 | `retry` 单元测试 |
| Push 413 | 非原子批次递归拆分 | `push` 单元测试 |
| Atomic Group | 拆批不破坏原子组；超大原子组可识别 | `push` 单元测试 |
| Cursor Recovery | Cursor 过期进入 Snapshot | `engine` 单元测试 |
| Pull Atomicity | 页应用与 Cursor 同事务 | SQLite Adapter 测试/代码路径 |
| Conflict | Pending 本地变更产生持久化冲突；可 Accept Remote/Keep Local/Discard | SQLite Adapter 测试与命令 |
| Tombstone | 远端删除写 Metadata 并阻止旧写入复活 | SQLite Adapter 测试/代码路径 |
| Snapshot | 分页 Staging、Resume、事务性 Finalize | Sync Core + SQLite Adapter |
| Token Refresh | 401 Single-Flight Refresh；轮换 Refresh Token | Transport + Cloud 回归 |
| Server Ownership | 请求不能指定任意 Cloud User；Cloud 按 Principal 隔离 | PostgreSQL Cloud 回归 |
| Scheduler | 本地变更、30 秒维护、5 分钟周期、Retry 到期 | Runtime 代码路径 |
| Frontend | 登录绑定选择、同步状态、冲突数量、手动同步 | ESLint、Unit、Production Build |
| Windows | Pure Core、Tauri Library、Frontend Build | `windows-client` CI Job |
| Linux/Desktop | Rustfmt、Contract、Core、SQLite、Clippy、Frontend | `linux-core-desktop-frontend` CI Job |
| Cloud/PostgreSQL | 认证、持久化、用户隔离、原有回归 | `cloud-postgresql` CI Job |

## 发布门禁

临时应用工作流只有在 Contract、纯同步核心、Tauri SQLite、两组 Clippy、前端 lint/unit/build 全部成功后，才生成正式代码提交并清理覆盖包。正式 `.github/workflows/epic05-windows-sync.yml` 继续在 Linux、PostgreSQL 和 Windows 三个平台验证。
