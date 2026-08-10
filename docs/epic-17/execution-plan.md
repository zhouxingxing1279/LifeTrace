# EPIC-17 安全、隐私与数据生命周期执行计划

> 状态：执行中  
> 日期：2026-08-10  
> 分支：`agent/epic-17-security-lifecycle`  
> 基线：`main@4804931476fbb824264030e2ca47239506c02354`  
> 目标：在不改变现有业务语义的前提下，为 LifeTrace 建立可验证的服务端安全基线、客户端安全约束和用户数据生命周期控制能力。

## 1. Epic 边界

本次严格对应路线图 EPIC-17：

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

当前代码已经具备部分 EPIC-17 基础：

- `services/cloud/src/lib.rs` 已存在 CORS 白名单能力，并允许 `x-csrf-token`。
- `services/cloud/src/routes/web_auth.rs` 已实现 HttpOnly Cookie Web Session 与 CSRF 校验入口。
- `services/cloud/src/config.rs` 已实现生产环境 Secret、Secure Cookie、HTTPS `PUBLIC_WEB_BASE_URL` 等 fail-closed 校验。
- EPIC-04 已提供登录限流、Session/设备撤销和公共设备短会话能力。

因此本次不会重复造一套认证系统，而是在现有认证基线上补齐：统一安全响应头、生产安全配置、统一数据导出/删除接口、敏感日志脱敏工具与安全回归测试。

## 3. 实现策略

### 阶段 A：服务安全基线

新增独立 `security` 模块：

- 所有响应增加 `X-Content-Type-Options: nosniff`
- 增加 `Referrer-Policy: no-referrer`
- 增加 `X-Frame-Options: DENY`
- 增加 `Permissions-Policy`
- 增加严格 CSP，默认禁止远程脚本和 framing
- 生产环境增加 HSTS
- 对敏感 API 默认增加 `Cache-Control: no-store`
- 保持现有 CORS allowlist 行为，不在未配置时退化为 `*`

配置层增加：

- 生产环境必须至少配置一个 HTTPS CORS Origin
- 禁止生产环境将 HTTP Origin 加入 CORS allowlist
- 保留现有 HTTPS base URL、Secure Cookie、Secret fail-closed 校验

### 阶段 B：数据生命周期 API

新增认证后的 `/api/v1/privacy/*` 路由：

- `GET /api/v1/privacy/export`：全量导出用户可迁移数据
- `GET /api/v1/privacy/export/{module}`：按模块导出
- `GET /api/v1/privacy/policy`：返回当前保留策略摘要
- `DELETE /api/v1/privacy/account`：注销账号并清理云端业务数据、会话、设备凭据和关联对象元数据

模块导出第一版覆盖当前云端已经稳定存在的领域：

- profile / account metadata
- finance
- mail metadata（不输出认证 Secret）
- sync metadata / entities
- devices / sessions 的可读摘要

原则：

- 导出不得包含密码哈希、Token hash、pepper、session raw token 或其他认证 Secret。
- 注销必须先撤销 Session/Token，再删除业务数据，最后标记/删除账号。
- 删除操作必须通过 Web CSRF 或 Bearer authentication 的现有认证边界。
- 文件对象删除在当前对象存储实现尚未完成时通过 Repository/cleanup hook 预留，不伪造已删除远端对象的成功结果。

### 阶段 C：敏感日志与客户端安全约束

新增通用脱敏模块：

- Authorization / Cookie / Set-Cookie
- access token / refresh token / csrf token
- password / secret / api key
- 完整邮件/通知正文等高敏字段

客户端侧增加可测试的安全策略：

- 浏览器长期 Token 不进入 `localStorage`
- 公共设备模式禁止持久化认证状态
- production build 不输出 debug 级敏感诊断
- 本地数据库保护与 OS keychain/secure storage 的职责边界写入文档；不在没有平台安全存储实现时以“自制加密”替代系统安全存储。

## 4. 数据删除顺序

账号注销采用 fail-closed 顺序：

```text
re-authenticated principal
        ↓
revoke active sessions/tokens
        ↓
delete user-owned business rows
        ↓
delete sync/outbox/change metadata
        ↓
delete file object metadata + invoke object cleanup hook
        ↓
delete devices / auth grants / audit-sensitive metadata according to policy
        ↓
deactivate/delete cloud user
```

任何关键阶段失败均返回错误，不返回“注销成功”。

## 5. 保留策略

本次把策略变成显式配置/文档，而不是隐含实现：

- 在线主数据：账号存在期间保留；注销时按删除流程清理
- Session/Token：撤销后不可继续使用
- 导入原始文件：默认仅用于导入任务，成功解析后按配置清理
- 通知/邮件原文：按配置保留，允许只保留结构化摘要
- 备份：备份不承诺即时物理擦除；必须记录最大保留窗口和恢复后的再次删除要求
- 审计日志：仅保留安全事件所需最小字段，禁止写入原始 Secret

## 6. 测试计划

### Rust 单元/集成测试

必须覆盖：

1. 普通响应包含安全响应头
2. 生产模式包含 HSTS
3. CSP 不允许任意远程 script/frame
4. CORS 未配置时不允许任意 Origin
5. 生产环境拒绝 HTTP CORS Origin
6. 生产环境继续拒绝弱 Secret / 非 Secure Cookie / HTTP base URL
7. 未认证访问 privacy export/delete 被拒绝
8. 导出结果不包含 password/token/secret 字段
9. 模块导出只返回指定模块
10. 账号注销撤销 Session 并清理用户数据
11. 删除失败时不得返回成功
12. 日志脱敏覆盖 Authorization、Cookie、password、token、secret

### 现有回归门禁

合并 `main` 前至少要求现有主线 CI 全绿，并重点检查：

```text
cargo test --manifest-path services/cloud/Cargo.toml
npm --prefix apps/desktop run lint
npm --prefix apps/desktop run test:unit
npm --prefix apps/desktop run web:build
npm --prefix apps/desktop run browser:build
```

以 GitHub Actions 实际 workflow 结果作为最终门禁；任一 required/相关 job 失败则不合并。

## 7. 文档交付

新增：

- `docs/epic-17/execution-plan.md`
- `docs/epic-17/security-architecture.md`
- `docs/epic-17/data-lifecycle.md`

更新：

- `docs/roadmap.md`：仅更新 EPIC-17 完成状态，不塞实现细节
- `docs/README.md`：加入 EPIC-17 文档入口
- `services/cloud/README.md`：补生产安全配置说明

## 8. 合并策略

1. 从最新 `main` 创建 `agent/epic-17-security-lifecycle`
2. **先提交本执行计划**
3. 按阶段实现代码与测试
4. 更新架构/生命周期文档
5. 创建 PR 到 `main`
6. 等待 GitHub Actions 完整执行
7. CI 失败则修复并重新跑
8. 只有全部相关测试通过后合并 `main`
9. 合并后再次核对 `main` 提交和文档状态

## 9. 完成定义

- [ ] 服务端统一安全响应头生效
- [ ] 生产 HTTPS/CORS/Secret 配置 fail-closed
- [ ] Web 写操作继续受到 CSRF 保护
- [ ] API/登录限流能力未回退
- [ ] 敏感字段具备统一脱敏规则
- [ ] 全量导出可用且可读
- [ ] 分模块导出可用
- [ ] 导出中不包含认证 Secret
- [ ] 账号注销撤销 Session 并按策略清理业务数据
- [ ] 文件对象删除边界明确且不会虚报成功
- [ ] 备份、通知原文、导入文件保留策略文档化
- [ ] 客户端 Token/公共设备/调试日志安全规则文档化并可测试
- [ ] EPIC-17 新增测试通过
- [ ] 主线现有回归测试通过
- [ ] PR CI 全绿
- [ ] 合并到 `main`
- [ ] 路线图与文档索引已更新
