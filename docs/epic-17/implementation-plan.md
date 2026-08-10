# LifeTrace EPIC-17：安全、隐私与数据生命周期执行方案

> 文档状态：实现完成，CI 验收通过，待合并  
> 创建日期：2026-08-10  
> 最近更新：2026-08-10  
> 执行分支：`epic-17-security-privacy-lifecycle`  
> 基线提交：`4804931476fbb824264030e2ca47239506c02354`  
> 验收代码提交：`930bf05afd832f9f77a56e51e0d7c5bfe2e3663a`  
> PR：`#51`  
> Roadmap 对应：EPIC-17 安全、隐私与数据生命周期

---

## 1. 目标

在不重复改造已经完成的 EPIC-04 认证体系、EPIC-19 日志基础设施的前提下，补齐 EPIC-17 当前缺口，并把现有安全能力纳入自动化回归验证，最终满足 Roadmap 的四项硬验收：

1. 数据导出可读且完整。
2. 注销后业务数据按策略清理。
3. 敏感接口无法匿名访问。
4. 日志中不存在 Token、密码和完整敏感正文。

本次工作采用“验收标准优先”的方式：已有能力先验证，缺口再实现，所有结果以代码、测试与部署配置为证据，不以 Roadmap 勾选本身作为完成依据。

## 2. 基线与实施结论

### 2.1 复用的既有能力

本次没有重复实现已成熟的安全基础设施，而是复用并回归验证：

- Rust/Axum Cloud 与 PostgreSQL 的生产 Fail Closed 认证路径。
- Access/Refresh Token、Session、设备撤销、Web Cookie/CSRF。
- CORS 显式 Origin 白名单。
- 登录限流与认证审计。
- Windows 凭据、Vault 等敏感凭据安全存储。
- 客户端结构化日志和脱敏基础设施。
- `cloud_users` 与核心业务表的用户归属及 `ON DELETE CASCADE` 关系。

### 2.2 本次补齐的缺口

- 新增统一用户数据导出 API，并支持白名单模块化导出。
- 新增高风险账号注销 API，并验证用户业务数据级联清理和 Token 失效。
- 新增机器可读数据保留策略 API 与正式策略文档。
- 新增 Cloud 统一安全响应头基线。
- 新增 EPIC-17 专项匿名访问、安全响应头与 PostgreSQL 生命周期测试。
- 服务端隐私模块数据库错误只记录粗粒度错误类别，不输出 SQL、Token、凭据或业务正文。
- 外部对象删除能力未具备时采取 fail closed，不允许数据库先删而外部对象失联。

## 3. 已实施功能

### 3.1 Cloud 安全基线

`services/cloud/src/lib.rs` 增加全局安全响应头：

- `Content-Security-Policy`
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: no-referrer`
- `Permissions-Policy`
- `Cross-Origin-Resource-Policy`
- `Strict-Transport-Security`
- `Cache-Control: no-store`

继续复用现有 CORS 白名单、Web CSRF、认证、登录限流和 Request ID 实现，不引入平行安全栈。

### 3.2 数据导出

新增受认证保护的接口：

```text
GET /api/v1/privacy/export
GET /api/v1/privacy/export?modules=<module,...>
GET /api/v1/privacy/policy
```

支持的导出模块：

```text
account
devices
grants
sync
mail
```

默认返回全部模块；指定模块时只返回白名单模块；未知模块返回 `400`。

导出包含用户可读业务数据与必要账号元数据，明确排除：

- password hash；
- Access/Refresh Token 原文；
- Session Secret；
- 邮件 credential ciphertext / nonce；
- 服务端 Secret 与签名密钥。

### 3.3 账号注销与云端数据删除

新增：

```text
DELETE /api/v1/privacy/account
```

删除流程：

1. 必须是已认证主体。
2. 必须提交精确确认短语 `DELETE MY ACCOUNT`。
3. 在 PostgreSQL 事务内检查外部文件对象引用。
4. 如存在尚不能可靠删除的外部 `storage_ref`，立即 fail closed 并回滚。
5. 将需长期保留的最小安全审计记录解除 user/session/device 标识关联，并压缩为账号已删除事件。
6. 删除 `cloud_users` 用户锚点。
7. 依赖外键级联删除设备、授权、Session、Token、同步实体、变更、Snapshot、邮件业务数据等在线用户数据。
8. 删除完成后旧 Access Token 无法再次认证。

### 3.4 数据保留策略

新增：

```text
docs/epic-17/data-retention-policy.md
```

策略覆盖：

- 在线业务数据；
- Session 与 Token；
- 通知原文；
- 导入文件与附件；
- 诊断日志；
- 最小安全审计；
- 历史加密备份；
- 数据导出；
- 账号注销。

备份不在注销时做原地重写，在线数据立即删除，历史加密备份按配置窗口自然淘汰；恢复流程不得把已注销账号重新投入在线服务。

## 4. 实际代码变更

实际落点：

```text
services/cloud/src/lib.rs
services/cloud/src/routes/mod.rs
services/cloud/src/routes/privacy.rs
services/cloud/tests/privacy_security.rs
services/cloud/tests/privacy_postgres.rs
docs/epic-17/implementation-plan.md
docs/epic-17/data-retention-policy.md
docs/roadmap.md
```

与初始预计相比，没有新增独立 `services/cloud/src/privacy.rs`：当前生命周期逻辑规模适合放在 `routes/privacy.rs`，避免不必要的抽象层。也没有永久修改现有 CI workflow；排错期间的临时诊断修改已完整撤销，最终工作流保持仓库原有定义。

## 5. 实际执行顺序

1. **先创建执行文档并单独提交**：`8568d88156d70c4082ae197b7ef939d0a8330d91`。
2. 审计现有认证、路由、数据库外键、日志与部署配置。
3. 实现 Cloud 安全响应头与隐私生命周期 API。
4. 实现全量/分模块导出。
5. 实现事务化账号注销、在线数据级联清理和 Session/Token 失效。
6. 补数据保留策略文档。
7. 补专项测试。
8. 创建 PR `#51` 并运行仓库真实 CI。
9. 第一轮 Cloud tests 发现测试断言误报：响应的 `secretsExcluded` 元数据会主动列出 `credentialCiphertext`，旧断言却在整个响应中禁止出现该名称；已将断言收敛到真实 `data` 区域，同时仍对实际 Token 值做整响应禁止检查。
10. Browser Web CI 进一步发现 Rust 格式未提交；按 CI 产出的 `rustfmt.patch` 精确修复 4 处纯格式差异。
11. 最终代码 HEAD `930bf05afd832f9f77a56e51e0d7c5bfe2e3663a` 的三条 PR 工作流全部通过。
12. 测试通过后回写本文和 Roadmap。
13. 文档版 HEAD 通过最终合并检查后，以非 squash 方式合并 PR，保留“文档先行”的提交历史。
14. 合并后复核 `main` 并在本文记录最终 merge SHA。

## 6. 验收测试矩阵与结果

| 验收项 | 验证方式 | 结果 |
| --- | --- | --- |
| 数据导出可读 | PostgreSQL 集成测试注册真实用户、插入同步实体后调用 export | 通过 |
| 分模块导出 | 白名单模块解析、未知模块 400 | 通过 |
| 导出不泄密 | `data` 区禁止 password/credential/session secret；实际 Access/Refresh Token 值禁止出现在整响应 | 通过 |
| 注销清理 | 删除后验证 `cloud_users`、`sync_entities`、`auth_sessions` 均为 0 | 通过 |
| Session/Token 失效 | 注销后用原 Bearer Token 再认证必须失败 | 通过 |
| 匿名访问 | privacy policy/export/account delete 无认证返回 401/403 | 通过 |
| 安全响应头 | CSP、nosniff、DENY、Referrer-Policy、HSTS、no-store 等断言 | 通过 |
| PostgreSQL 回归 | EPIC-03 Cloud tests + Compose PostgreSQL smoke | 通过 |
| Rust 静态检查 | rustfmt + clippy `-D warnings` | 通过 |
| Browser 回归 | lint、unit test、Web build、browser build、Cloud auth regression | 通过 |
| Windows/Linux 回归 | Windows core/desktop/frontend、Linux core/desktop/frontend、Cloud PostgreSQL | 通过 |
| Docker | Cloud Docker build | 通过 |

最终 CI 证据（代码 HEAD `930bf05afd832f9f77a56e51e0d7c5bfe2e3663a`）：

- `Browser Web` run `31371942587`：success。
- `EPIC-05 Windows Sync` run `31371942585`：success。
- `EPIC-03 PostgreSQL` run `31371942581`：success。

## 7. 完成判定

- [x] 本文已先于实现代码提交。
- [x] 全量与分模块导出已实现并通过测试。
- [x] 账号注销与在线业务数据清理已实现并通过事务/级联测试。
- [x] 敏感隐私路由匿名访问测试通过。
- [x] 隐私模块错误输出不记录 SQL、Token、凭据或完整业务正文，既有客户端脱敏回归保持通过。
- [x] 安全响应头、CORS、CSRF、认证与限流既有能力未被回归破坏。
- [x] 数据保留策略文档完成。
- [x] 项目 PR 级 Browser、Windows/Linux、Cloud/PostgreSQL、Docker 门禁全部通过。
- [ ] Roadmap 已回写最终完成状态。
- [ ] 变更已通过 PR 合并到 `main`。

## 8. 回滚原则

- 所有数据库删除操作必须在单事务内执行，失败即整体回滚。
- 不新增不可逆 schema 破坏性迁移。
- 安全头若导致未来同源 HTML 资源加载回归，应为对应 Web origin 设计独立 CSP，而不是整体关闭 API 安全头。
- 导出 API 只读，可独立回退而不影响同步主链路。
- 注销 API 在出现任何无法确认的外部对象删除状态时必须 fail closed，不得报告“已全部删除”。

---

## 9. 实施记录

### 实际变更

已完成安全响应头、隐私数据导出、机器可读保留策略、事务化账号注销、级联在线数据删除、Session/Token 失效验证、专项安全测试及正式保留策略文档。

### 测试结果

最终代码 HEAD 的三条 PR 工作流均为 `success`；测试覆盖 Rust 格式、合同测试、Cloud 全量测试、Clippy、Docker build、Compose PostgreSQL 烟测、Browser lint/unit/build、Windows/Linux 客户端与同步核心回归。

### 与计划的偏差

1. **对象存储**：仓库目前尚未把 EPIC-12 对象存储作为正式生产依赖，因此没有伪造对象删除器。若账号存在非空外部 `storage_ref`，注销事务会直接拒绝并回滚；待 EPIC-12 上线后接入真实对象删除 adapter。
2. **模块拆分**：没有额外创建 `services/cloud/src/privacy.rs`，避免当前规模下的重复抽象。
3. **CI**：没有永久修改工作流。排查测试误报时曾临时增强失败 annotation，根因确认后已恢复原 workflow；随后 Browser CI 捕获并推动提交 `rustfmt` 结果。

### 最终状态

实现完成，代码 CI 验收通过。Roadmap 回写和 PR 合并进行中；合并完成后记录最终 `main` SHA。
