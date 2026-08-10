# LifeTrace EPIC-17 生产部署安全基线

> 版本：1  
> 日期：2026-08-10  
> 适用范围：LifeTrace Cloud、Browser、Windows/Desktop 生产部署

## 1. 网络与 HTTPS

- 公网只暴露 TLS 反向代理入口，Cloud `8787` 端口保持私网可达。
- `deploy/cloud/Caddyfile.example` 是生产 TLS 基线；使用真实 `LIFETRACE_DOMAIN` 时由 Caddy 获取和续期证书。
- 生产配置必须满足 `PUBLIC_WEB_BASE_URL=https://...`，且 `AUTH_COOKIE_SECURE=true`。
- Cloud 响应统一设置 HSTS、CSP、`nosniff`、`DENY` frame policy、`no-referrer`、Permissions-Policy、CORP 与 `Cache-Control: no-store`。

## 2. CORS、CSRF 与 XSS

- CORS 只允许 `CORS_ALLOWED_ORIGINS` 显式白名单；生产环境不得使用任意 Origin。
- Browser 写操作继续使用 HttpOnly Session Cookie + CSRF Token + Origin 校验。
- API origin 的 CSP 默认 `default-src 'none'`，不把 API origin 当作任意活动内容宿主。
- 邮件 HTML 在进入客户端展示前必须走既有 sanitized 字段/展示边界，禁止把不可信 HTML 当作脚本执行。

## 3. API 与登录限流

Cloud 有两层独立限流：

1. **通用 API 限流**：覆盖 `/api/`，按可信客户端 IP 计数，默认 `600` 请求 / `60` 秒；超过限制返回 `429`、`LIFETRACE_RATE_LIMITED` 和 `Retry-After`。
2. **登录限流**：沿用认证服务的 PostgreSQL 持久化账号/IP 限流和锁定策略，防止爆破。

可配置项：

```text
API_RATE_LIMIT_REQUESTS
API_RATE_LIMIT_WINDOW_SECONDS
AUTH_LOGIN_ACCOUNT_LIMIT
AUTH_LOGIN_IP_LIMIT
AUTH_LOGIN_WINDOW_SECONDS
AUTH_LOCKOUT_SECONDS
```

反向代理场景只信任 `AUTH_TRUSTED_PROXY_CIDRS` 指定的代理来源，不能无条件信任外部伪造的 Forwarded/X-Forwarded-For。

## 4. Secret 管理

生产运行时 Secret 不得提交到 Git，不得写入普通日志，也不得构建进镜像。

Cloud 主配置支持直接环境变量和同名 `*_FILE` 两种读取方式；生产 Docker Compose 示例使用文件挂载注入以下 Secret：

```text
DATABASE_URL_FILE
CURSOR_SIGNING_KEY_FILE
PAGE_TOKEN_SIGNING_KEY_FILE
AUTH_PASSWORD_PEPPER_FILE
AUTH_TOKEN_HASH_PEPPER_FILE
MAIL_CREDENTIAL_KEY_FILE
```

邮件授权凭据自身使用 AES-256-GCM 加密后入库，`MAIL_CREDENTIAL_KEY`/`MAIL_CREDENTIAL_KEY_FILE` 只存在于服务进程运行时安全边界。

DeepSeek 等可选第三方 API Key 同样属于 Secret：生产环境应由平台 Secret Manager/受保护运行时变量注入，不得写在 `.env.example` 的真实值、源码、CI 日志或诊断包中。若部署平台支持 Secret 文件，优先使用文件挂载并在启动层映射为进程私有变量。

## 5. PostgreSQL 最小权限

生产环境分离两类数据库身份：

- **迁移/Owner 角色**：仅发布/迁移阶段使用，可执行 DDL；不作为常驻 Cloud 运行身份。
- **`lifetrace_app` 应用角色**：Cloud 常驻进程使用，只保留 CONNECT、schema USAGE、业务表 DML 和必要 sequence 权限。

生产 `MIGRATION_ON_STARTUP=false`。发布阶段使用迁移身份执行迁移后，再运行：

```text
deploy/cloud/postgres/least-privilege.sql
```

Cloud 生产配置还会拒绝常见超级用户账号（如 `postgres`、`root`）作为运行时 `DATABASE_URL` 用户，防止误把管理员凭据当应用凭据。

## 6. Browser 公共设备隐私模式

Browser 登录支持 `publicDevice`：

- 不签发可长期持久化的 Refresh Token；
- Web Session 使用独立、较短的公共设备 TTL；
- Session Cookie 不设置持久化 `Max-Age`，浏览器会话结束后由浏览器清理；
- Cookie 保持 HttpOnly，写操作仍要求 CSRF；
- 用户可通过退出/全局退出立即撤销 Session。

公共设备模式的目标是降低共享机器残留认证状态风险，而不是替代操作系统账户隔离。

## 7. Desktop 本地数据保护

Desktop SQLite 统一通过单一连接入口打开，并启用：

```text
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=5000;
PRAGMA secure_delete=ON;
PRAGMA trusted_schema=OFF;
PRAGMA temp_store=MEMORY;
```

其中 `secure_delete` 降低被删除敏感页残留，`trusted_schema=OFF` 收紧不可信 schema 行为，临时数据尽量保持内存。Token、第三方凭据、相册密钥等高价值 Secret 不以普通 SQLite 明文业务字段保存，继续使用 OS 凭据存储/Vault/专用加密边界。

SQLite 本身不是全盘加密替代品；设备丢失威胁模型仍依赖 Windows 账户隔离和 BitLocker/设备加密等操作系统能力。

## 8. 日志与诊断

- 生产环境不启用用于打印请求正文、Token、Cookie、密码、第三方凭据的 debug 日志。
- 隐私生命周期模块数据库失败只记录粗粒度错误类型，不记录 SQL、行内容或凭据。
- EPIC-19 诊断日志继续执行结构化记录、轮转和敏感字段脱敏。
- 邮件正文、健康正文、完整通知原文不得进入普通日志；如排障确需用户数据，必须由用户主动导出并单独处理，不能通过“提高日志级别”绕过数据边界。

## 9. 数据删除与备份

- `DELETE /api/v1/privacy/account` 在 PostgreSQL 单事务内清理在线用户数据。
- Session/Token 随账号删除立即失效。
- 如果存在非空外部对象 `storage_ref` 且真实对象删除 adapter 尚未成功执行，注销必须 fail closed 并回滚，不能虚报删除完成。
- 历史加密备份不在注销时原地改写；按配置保留窗口淘汰，恢复流程必须尊重已注销状态。

## 10. 发布前检查

生产发布至少验证：

- [ ] 公网只暴露 HTTPS 入口，Cloud 端口不直接暴露。
- [ ] `PUBLIC_WEB_BASE_URL` 为 HTTPS，Secure Cookie 开启。
- [ ] CORS 仅包含真实前端 Origin。
- [ ] 主 Secret 使用平台 Secret/Docker Secret 注入，仓库无真实值。
- [ ] Cloud `DATABASE_URL` 使用非超级用户的最小权限应用角色。
- [ ] API 429 和登录限流回归测试通过。
- [ ] 匿名访问隐私/同步/财务/邮件等敏感接口被拒绝。
- [ ] 账号注销、数据导出、Token 失效测试通过。
- [ ] rustfmt、clippy、Cloud/PostgreSQL、Browser、Windows/Linux、Docker/Compose CI 全部通过。
