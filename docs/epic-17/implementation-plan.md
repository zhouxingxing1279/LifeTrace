# LifeTrace EPIC-17：安全、隐私与数据生命周期执行方案

> 文档状态：实施中  
> 创建日期：2026-08-10  
> 执行分支：`epic-17-security-privacy-lifecycle`  
> 基线提交：`4804931476fbb824264030e2ca47239506c02354`  
> Roadmap 对应：EPIC-17 安全、隐私与数据生命周期

---

## 1. 目标

在不重复改造已经完成的 EPIC-04 认证体系、EPIC-19 日志基础设施的前提下，补齐 EPIC-17 当前缺口，并把现有安全能力纳入自动化回归验证，最终满足 Roadmap 的四项硬验收：

1. 数据导出可读且完整。
2. 注销后业务数据按策略清理。
3. 敏感接口无法匿名访问。
4. 日志中不存在 Token、密码和完整敏感正文。

本次工作采用“验收标准优先”的方式：已有能力先验证，缺口再实现，所有结果以代码、测试与部署配置为证据，不以 Roadmap 勾选本身作为完成依据。

## 2. 当前基线与差距

### 2.1 已有能力，原则上只补测试或加固

- Rust/Axum Cloud 与 PostgreSQL 已建立生产 Fail Closed 的认证路径。
- Access/Refresh Token、Session、设备撤销、Web Cookie/CSRF 已由 EPIC-04 提供。
- CORS 已支持显式 Origin 白名单。
- 登录限流与认证审计已有持久化基础。
- Windows 凭据、Vault 等敏感凭据已有安全存储能力。
- 客户端结构化日志与脱敏能力已有 EPIC-19 基础。
- `cloud_users` 与核心同步表使用用户归属关系，主要表支持 `ON DELETE CASCADE`。

### 2.2 本次必须补齐的缺口

- 缺少统一的用户数据导出 API 与模块化导出能力。
- 缺少高风险账号注销 API，以及可验证的用户业务数据级联清理。
- 缺少统一的数据保留/删除策略声明和机器可读策略接口。
- 缺少面向 Cloud 的统一安全响应头基线（CSP、nosniff、frame、referrer、permissions 等）。
- 缺少 EPIC-17 专项匿名访问回归矩阵。
- 缺少服务端日志敏感字段二次兜底测试与静态扫描门禁。
- 对已存在的 HTTPS/CORS/CSRF/限流/Secret/最小权限能力需要形成可审计的完成证据。

## 3. 实施范围

### 3.1 Cloud 安全基线

计划：

- 在 Axum 全局层加入统一安全响应头。
- 保持 CORS 为显式白名单；生产环境不允许隐式放开跨域。
- 复用现有 Web CSRF 保护与认证限流，不另起并行实现。
- 对需要认证的业务路由建立匿名访问测试矩阵。
- 增加日志/错误输出敏感字段扫描，覆盖：`Authorization`、Cookie、Access Token、Refresh Token、Password、API Key、邮件正文、健康正文等。
- 对部署文档补充 HTTPS、Secret 注入和 PostgreSQL 最小权限约束。

### 3.2 数据导出

新增受认证保护的隐私/生命周期 API：

```text
GET /api/v1/privacy/export
GET /api/v1/privacy/export?modules=<module,...>
GET /api/v1/privacy/policy
```

导出格式使用可读 JSON，并包含：

- schema/version 与导出时间；
- 当前用户账号的非机密元数据；
- 设备与应用授权元数据；
- 用户同步实体（按 `entity_type` 分组）；
- 邮件行动中心等 Cloud 专属用户业务数据的可导出表示；
- 数据保留策略说明。

明确禁止导出：

- password hash；
- refresh/access token 原文或哈希；
- session secret；
- 加密凭据密文及密钥材料；
- 服务端 Secret。

模块化导出通过白名单模块选择实现；未知模块返回明确的参数错误，不静默忽略。

### 3.3 账号注销与云端删除

新增高风险受认证 API：

```text
DELETE /api/v1/privacy/account
```

执行策略：

1. 必须已有有效认证主体。
2. 请求必须显式携带删除确认短语，避免误触。
3. 数据库事务内删除 `cloud_users` 用户锚点。
4. 依赖外键级联清理设备、Session、Token、同步实体、变更日志、Snapshot、邮件数据等用户业务数据。
5. 对未被外键覆盖的用户表在同一事务中显式清理。
6. 注销完成后当前及其他 Session/Refresh Token 立即失效。
7. API 返回不包含已删除用户敏感信息的完成摘要。
8. 备份不做在线物理擦除；由保留策略到期淘汰，并在策略文档中明确说明。

如果当前仓库尚不存在正式 Cloud 对象存储，本次会建立“对象删除钩子/策略边界”，不伪造 S3 删除成功；待 EPIC-12 对象存储落地后接入真实对象删除实现。

### 3.4 数据保留策略

统一定义并文档化至少以下生命周期：

- 在线业务数据：账号存续期间保留，用户主动删除/注销时在线副本清理。
- Session/Refresh Token：撤销后立即不可用；数据库记录按安全审计需要保留最小必要元数据。
- 通知原文：仅在业务确有需要时保留，默认按最小必要原则；禁止写入普通日志。
- 导入原始文件：导入完成后按用户可见策略保留，可主动删除；不允许无限期无说明保留。
- 诊断日志：按 EPIC-19 的轮转和保留策略处理，敏感内容脱敏。
- 备份：注销不会修改历史备份；备份在既定保留窗口内自然淘汰，恢复流程必须尊重已注销账号的删除状态。

## 4. 预计代码变更

主要落点：

```text
services/cloud/src/lib.rs
services/cloud/src/routes/mod.rs
services/cloud/src/routes/privacy.rs                 # 新增
services/cloud/src/privacy.rs                        # 新增，生命周期业务逻辑
services/cloud/tests/privacy.rs                      # 新增专项集成测试
services/cloud/tests/security_headers.rs             # 新增安全头测试
scripts/                                             # 敏感日志/源码扫描门禁（如现有脚本可复用则扩展）
.github/workflows/                                   # 将专项检查接入现有 CI（按当前工作流结构修改）
docs/epic-17/implementation-plan.md                  # 本文，实施后回写结果
docs/epic-17/data-retention-policy.md                # 生命周期策略
docs/roadmap.md                                      # 测试通过后更新完成状态
```

如代码审计发现已有同等模块，将优先扩展已有实现，避免产生重复层。

## 5. 执行顺序

严格按以下顺序执行：

1. **执行文档先行**：创建本文并单独提交，不混入任何代码变更。
2. 审计现有认证、路由、数据库外键、日志与部署配置。
3. 实现 Cloud 安全响应头与隐私生命周期服务。
4. 实现全量/分模块数据导出。
5. 实现账号注销、Session 撤销与用户在线数据级联删除。
6. 补数据保留策略与部署安全说明。
7. 补专项测试与 CI 门禁。
8. 执行格式化、静态检查、单元/集成测试以及项目现有全量门禁。
9. 所有测试通过后，回写本文的实际变更、测试结果、风险与偏差。
10. 更新 `docs/roadmap.md` / 项目完成状态证据。
11. 创建 PR，等待/核验 required checks 全部成功。
12. **仅在测试与 PR 检查通过后合并到 `main`**。
13. 合并后复核 `main` SHA 与文档状态。

## 6. 验收测试矩阵

| 验收项 | 自动化验证 |
| --- | --- |
| 数据导出完整 | 构造多种用户实体；导出后核对账号元数据、设备、同步实体、Cloud 专属业务数据数量与内容 |
| 分模块导出 | 指定模块仅返回白名单模块；未知模块返回 400 |
| 导出不泄密 | 断言响应中不存在 password hash、token、secret、credential ciphertext |
| 注销清理 | 创建用户及关联设备、Session、Token、同步实体、邮件数据；注销后逐表验证不存在在线用户业务数据 |
| Session 失效 | 注销前 Token 可访问；注销后原 Token 无法访问敏感 API |
| 匿名访问 | 对 export、account delete、sync、finance、mail、assistant 等敏感接口逐项断言 401/403 |
| 安全响应头 | 核对 CSP、X-Content-Type-Options、frame policy、Referrer-Policy、Permissions-Policy 等 |
| CORS | 非白名单 Origin 不获得允许跨域响应头；白名单按配置工作 |
| CSRF | Cookie 写操作缺少/错误 CSRF Token 时拒绝 |
| 登录限流 | 复用现有认证测试并保证回归通过 |
| 日志脱敏 | 构造 Token、密码、Cookie、邮件/健康正文样例，确保输出为 `[REDACTED]` 或完全不记录 |
| 全量回归 | `npm run test:all`；同时执行现有 CI 中的 lint/build/security 检查 |

## 7. 完成判定

EPIC-17 只有同时满足以下条件才标记完成：

- [ ] 本文已先于实现代码提交。
- [ ] 全量与分模块导出已实现并通过测试。
- [ ] 账号注销与在线业务数据清理已实现并通过事务/级联测试。
- [ ] 敏感路由匿名访问测试全部通过。
- [ ] 日志敏感信息扫描与脱敏测试通过。
- [ ] 安全响应头、CORS、CSRF、限流回归通过。
- [ ] 数据保留策略文档完成。
- [ ] 项目现有全量测试与 CI required checks 全部通过。
- [ ] Roadmap 与本执行文档已回写实际完成状态。
- [ ] 变更已通过 PR 合并到 `main`。

## 8. 回滚原则

- 所有数据库删除操作必须在单事务内执行，失败即整体回滚。
- 不新增不可逆 schema 破坏性迁移；若新增表/索引，必须保证旧客户端可继续运行。
- 安全头若导致 Web 资源加载回归，优先缩小 CSP 到已知资源白名单，而不是整体关闭安全头。
- 导出 API 只读，出现问题可独立回退路由而不影响同步主链路。
- 注销 API 在出现任何无法确认的外部对象删除状态时必须 fail closed，不得报告“已全部删除”。

---

## 9. 实施记录

> 本节在实现、测试完成后回写；在首个“文档先行”提交中保持为空结果状态。

### 实际变更

待实施。

### 测试结果

待实施。

### 与计划的偏差

待实施。

### 最终状态

实施中。
