# EPIC-17 安全、隐私与数据生命周期执行计划

> 状态：已完成  
> 日期：2026-08-10  
> 首个执行文档提交：`3694ffaa95227af0981e4f08b0cc28b2be46c0df`  
> 实现分支：`agent/epic-17-security-lifecycle`  
> 基线：`main@4804931476fbb824264030e2ca47239506c02354`  
> 最终测试 Head：`5111cf38759ae767fe3ba41354c1b530ec686ff7`  
> 合并提交：`a4df9c8d205392c49fc0fd0063719af5778caa25`  
> 目标：在不改变现有业务语义的前提下，为 LifeTrace 建立可验证的服务端安全基线、客户端安全约束和用户数据生命周期控制能力。

## 1. Epic 边界

本次严格对应 EPIC-17：

### 服务安全

- 全站 HTTPS 生产约束
- CORS 白名单
- CSP
- CSRF 防护
- XSS 防护基线
- API / 登录限流
- 安全响应头
- Secret 管理
- 数据库最小权限约束

### 客户端安全

- Token 安全存储约束
- 应用权限最小化
- 敏感日志脱敏
- 公共设备隐私模式
- 本地数据库保护策略
- 调试日志关闭策略

### 数据生命周期

- 全量导出
- 分模块导出
- 账号注销
- 云端业务数据删除
- 文件对象删除接口/契约
- Session 撤销
- 备份保留说明
- 通知原文保留期
- 导入文件保留策略

## 2. 现状审计结论

执行前代码已经具备部分 EPIC-17 基础：

- `services/cloud/src/lib.rs` 已存在 CORS 白名单能力，并允许 `x-csrf-token`。
- `services/cloud/src/routes/web_auth.rs` 已实现 HttpOnly Cookie Web Session 与 CSRF 校验入口。
- `services/cloud/src/config.rs` 已实现生产环境 Secret、Secure Cookie、HTTPS `PUBLIC_WEB_BASE_URL` 等 fail-closed 校验。
- EPIC-04 已提供登录限流、Session/设备撤销和公共设备短会话能力。

因此本次没有重复建设认证系统，而是在现有认证基线上补齐统一安全响应头、生产安全配置、通用 API 限流、统一数据导出/删除接口、敏感日志脱敏与安全回归测试。

## 3. 实现策略

### 阶段 A：服务安全基线

新增独立 `security` 模块并接入应用入口：

- 所有响应增加 `X-Content-Type-Options: nosniff`
- 增加 `Referrer-Policy: no-referrer`
- 增加 `X-Frame-Options: DENY`
- 增加 `Permissions-Policy`
- 增加严格 CSP，默认禁止远程脚本和 framing
- 生产环境增加 HSTS
- API 响应增加 `Cache-Control: no-store`
- 保持现有 CORS allowlist 行为，不在未配置时退化为 `*`
- 生产环境禁止 wildcard、`null` 和 HTTP CORS Origin
- 生产运行时要求 `MIGRATION_ON_STARTUP=false`，将 migration 权限与运行时 DML 权限分离

同时新增 `/api/*` 通用限流；登录端继续沿用 EPIC-04 更严格的 credential-aware 限流。通用限流复用 `AUTH_TRUSTED_PROXY_CIDRS` 的可信代理边界，避免无条件信任可伪造的 `X-Forwarded-For`。

### 阶段 B：数据生命周期 API

新增认证后的 `/api/v1/privacy/*` 路由：

- `GET /api/v1/privacy/export`：全量导出当前 Principal 有读取权限的数据
- `GET /api/v1/privacy/export/{module}`：按模块导出
- `GET /api/v1/privacy/policy`：返回当前保留策略摘要
- `DELETE /api/v1/privacy/account`：注销账号并清理云端用户所有权根数据

导出原则：

- 继续执行现有 Scope 权限，不把隐私导出变成跨领域权限旁路。
- 用户账号字段采用 allowlist。
- 不导出密码哈希、Token hash、raw session token、pepper、邮件认证密文等认证 Secret。

删除原则：

- Web Session 删除操作继续执行 CSRF + Origin 校验；Bearer 调用走原有 Token 认证边界。
- 先撤销活动 Session，再删除 `cloud_users` 用户所有权根，由 PostgreSQL 外键级联清理设备、同步、认证和邮件数据。
- 删除过程使用事务，关键步骤失败不能返回成功。
- 当前服务尚无通用对象存储 cleanup provider；检测到非空外部 `storage_ref` 时 fail-closed 返回错误，不虚报对象已经物理删除。

### 阶段 C：敏感日志与客户端安全约束

云端新增递归结构化脱敏，覆盖：

- Authorization / Cookie / Set-Cookie
- access token / refresh token / csrf token
- password / secret / api key
- credential ciphertext 等认证材料

客户端在现有 observability 基础上进一步收紧：

- 完整邮件正文、通知正文、raw body 等高敏内容强制脱敏
- production build 中 debug 日志在创建前直接丢弃
- URL 日志继续移除 query string 与 fragment

同时核实现有认证持久化边界：

- Access Token 仅保存在运行时内存
- Refresh Token 通过 Tauri 原生桥接写入 Windows Credential Manager
- Web Storage 不保存 Access/Refresh Token
- Tauri capability 维持当前功能所需最小权限
- 本地 SQLite 的保护边界明确为 OS 用户 ACL、设备磁盘加密和应用权限隔离；本 Epic 不虚假声称已经实现 SQLCipher

## 4. 数据删除顺序

账号注销采用 fail-closed 顺序：

```text
authenticated principal
        ↓
Bearer auth OR Web Session + CSRF
        ↓
require account:write
        ↓
检查无法由当前服务清理的外部对象
        ↓
BEGIN transaction
        ↓
revoke active sessions
        ↓
DELETE cloud_users
        ↓
FK ON DELETE CASCADE 清理用户所属数据
        ↓
COMMIT
        ↓
清除浏览器 Session Cookie
```

任何关键阶段失败均返回错误，不返回“注销成功”。

## 5. 保留策略

本次把策略变成显式文档，而不是隐含实现：

- 在线主数据：账号存在期间保留；注销时按删除流程清理
- Session/Token：撤销后不可继续使用，账号删除时级联清理
- 导入原始文件：默认属于设备本地导入资产；临时文件完成解析后按配置清理
- 通知/邮件原文：只在对应业务功能需要时保留；禁止复制到普通诊断日志
- 备份：在线删除不等于历史备份块瞬时物理擦除；部署必须定义最大保留窗口，恢复旧备份后需重新应用删除动作
- 审计日志：仅保留安全调查需要的最小字段，禁止写入原始 Secret 或完整敏感正文

## 6. 测试与合并门禁

最终候选 Head：`5111cf38759ae767fe3ba41354c1b530ec686ff7`。

合并前 GitHub Actions 三条相关工作流全部通过：

1. Browser Web `31372513170`：通过
   - frontend lint
   - unit tests
   - Web build
   - Browser build
   - cloud Rust format/tests/clippy
2. EPIC-03 PostgreSQL `31372513618`：通过
   - format/contracts/cloud tests/clippy
   - Docker build
   - PostgreSQL Compose smoke
3. EPIC-05 Windows Sync `31372513662`：通过
   - cloud PostgreSQL regression
   - Linux core/desktop/frontend
   - Windows pure core/desktop tests
   - Windows frontend build

PR #50 仅在以上最终 Head 全绿后解除 Draft 并合并。

## 7. 文档交付

新增：

- `docs/epic-17/execution-plan.md`
- `docs/epic-17/security-architecture.md`
- `docs/epic-17/data-lifecycle.md`
- `docs/epic-17/completion-report.md`

更新：

- `docs/README.md`：加入 EPIC-17 文档入口
- `services/cloud/README.md`：补生产安全配置、隐私 API 和删除语义
- 本执行文档：记录最终测试 Head、合并提交和完成状态

`docs/roadmap.md` 保留 Epic 的产品级任务定义，不在执行收尾中塞入实现细节；EPIC-17 的实际完成证据以本目录的执行计划、架构文档、生命周期文档和完成报告为准。

## 8. 实际执行顺序

1. 从 `main@4804931476fbb824264030e2ca47239506c02354` 创建 `agent/epic-17-security-lifecycle`
2. **先提交本执行计划**：`3694ffaa95227af0981e4f08b0cc28b2be46c0df`
3. 按阶段实现代码与新增测试
4. 更新安全架构与数据生命周期文档
5. 创建 Draft PR #50
6. 使用 GitHub Actions 持续验证并修复 rustfmt 等门禁问题
7. 冻结最终候选 Head `5111cf38759ae767fe3ba41354c1b530ec686ff7`
8. 确认 Browser Web、EPIC-03 PostgreSQL、EPIC-05 Windows Sync 全绿
9. 解除 Draft，并使用精确 Head SHA 合并到 `main`
10. 合并提交：`a4df9c8d205392c49fc0fd0063719af5778caa25`
11. 关闭重复并行 PR #51，避免第二套重叠实现被再次合并
12. 核对 `main` 和文档状态并完成本执行文档收尾

## 9. 完成定义

- [x] 服务端统一安全响应头生效
- [x] 生产 HTTPS/CORS/Secret 配置 fail-closed
- [x] Web 写操作继续受到 CSRF 保护
- [x] 通用 API 限流与登录限流同时存在且未回退
- [x] 敏感字段具备统一脱敏规则
- [x] 全量导出可用且可读
- [x] 分模块导出可用
- [x] 导出中不包含认证 Secret
- [x] 账号注销撤销 Session 并按策略清理云端业务数据
- [x] 文件对象删除边界明确且不会虚报成功
- [x] 备份、通知/邮件原文、导入文件保留策略文档化
- [x] 客户端 Token/公共设备/调试日志安全规则文档化并可测试
- [x] EPIC-17 新增测试通过
- [x] 主线相关回归测试通过
- [x] PR 最终候选 Head 的 CI 全绿
- [x] PR #50 合并到 `main`
- [x] 文档索引、架构文档、生命周期文档和执行结果已更新
