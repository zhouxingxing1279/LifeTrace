# EPIC-17 安全架构

> 状态：已实现  
> 更新日期：2026-08-10  
> 适用范围：LifeTrace Cloud、Windows/Tauri 客户端与 Web/PWA 认证边界

## 1. 安全目标

EPIC-17 不替换 EPIC-04 已完成的认证系统，而是在它之上建立统一的安全基线：

- 网络入口只允许可信 HTTPS 部署；
- 浏览器 Session 使用 HttpOnly Cookie，写操作执行 CSRF 校验；
- 原生客户端使用短期 Access Token，并把长期凭据隔离到系统安全凭据存储；
- CORS、CSP、安全响应头和生产配置采用 fail-closed；
- `/api/*` 建立通用滥用限流，登录等高风险端点继续使用更严格的认证限流；
- 业务访问继续服从 App Scope 和用户所有权；
- Secret、Token、Cookie、密码和完整敏感正文不能进入普通诊断日志；
- 数据库运行身份与 migration 身份分离，降低运行时数据库权限；
- 数据导出和账号删除是认证后的显式用户操作。

## 2. 信任边界

```text
Browser / PWA
  └─ HTTPS + HttpOnly Session + CSRF
                    │
Windows / Tauri     │
  └─ Access Token ──┼────> LifeTrace Cloud / Axum
                    │        ├─ API rate limit
                    │        ├─ Auth / Scope
                    │        ├─ Security headers
                    │        ├─ Privacy API
                    │        └─ Domain / Sync API
                    │                 │
                    │                 └────> PostgreSQL
                    │                         └─ user ownership + FK cascade
                    │
                    └────> external object storage (future provider boundary)
```

当前云端尚未实现通用对象字节存储。数据库中如果出现非空外部对象引用，账号删除会拒绝返回成功，直到存在真正的对象清理 Provider；这是有意的 fail-closed 行为。

## 3. 已继承的 EPIC-04 控制

以下能力在 EPIC-17 之前已存在，本次保留并作为安全架构的一部分：

- Argon2id 密码哈希；
- Access Token / Refresh Token 与轮换；
- 登录限流；
- App Scope；
- 设备注册、撤销和远程退出；
- Secure / HttpOnly / SameSite Web Session Cookie；
- Web Session CSRF 校验；
- 公共设备短 Session；
- 生产 Secret、Cookie、`PUBLIC_WEB_BASE_URL` 的 fail-closed 配置校验。

EPIC-17 的原则是补齐横切安全策略，不建立第二套认证逻辑。

## 4. HTTP 安全基线

`services/cloud/src/security.rs` 对 API 响应统一增加：

| Header | 策略 | 目的 |
| --- | --- | --- |
| `X-Content-Type-Options` | `nosniff` | 禁止 MIME sniffing |
| `Referrer-Policy` | `no-referrer` | 避免 URL 信息通过 Referer 泄漏 |
| `X-Frame-Options` | `DENY` | 禁止被 frame，降低 clickjacking 风险 |
| `Permissions-Policy` | 禁用 camera/microphone/geolocation | API 服务默认不申请浏览器敏感能力 |
| `Content-Security-Policy` | `default-src 'none'` 等严格策略 | API 不加载任意远程脚本/Frame |
| `Cache-Control` | `no-store` | 避免认证/个人数据被中间缓存 |
| `Strict-Transport-Security` | 仅 production | 强制浏览器后续使用 HTTPS |

当前 CSP：

```text
default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'
```

这是针对 API 服务而不是静态站点的策略，因此可以比普通前端页面更严格。

## 5. API 与登录限流

`services/cloud/src/api_rate_limit.rs` 为 `/api/*` 增加应用级固定窗口限流：

- 默认每个传输层客户端 IP 600 请求/分钟；
- 真实服务从 Axum `ConnectInfo<SocketAddr>` 读取 peer IP，不信任客户端可伪造的 forwarded-IP 请求头；
- 超限返回 HTTP `429`、`LIFETRACE_RATE_LIMITED` 和 `Retry-After`；
- `/health/*` 不进入通用 API 配额，避免健康探针与业务流量互相影响；
- 内部计数表会做过期清理，避免长期无界增长。

这一层是通用滥用保护，不替代登录防爆破。认证端点继续使用 EPIC-04 的 credential-aware 登录限流，因此形成“全 API 基线 + 高风险端点更严格规则”的两层模型。

生产环境仍建议在公网边缘增加 WAF/反向代理限流和连接保护；应用级限流是纵深防御，不应作为唯一抗 DDoS 手段。

## 6. HTTPS 与 CORS

### 生产 HTTPS

生产环境应由可信反向代理、Ingress 或负载均衡器终止 TLS。应用层同时要求：

- `PUBLIC_WEB_BASE_URL` 使用 HTTPS；
- Session Cookie 开启 Secure；
- 已配置的 CORS Origin 必须是显式 HTTPS Origin；
- 禁止 `*`、`null` 和 HTTP Origin；
- production 响应返回 HSTS。

本地开发可以继续使用 `http://localhost`，上述限制只在 production fail-closed。

### CORS

未配置 Origin 时不会退化成 `*`。配置后只允许 allowlist 中的 Origin，并且仅放行声明的 Method/Header；需要 Cookie 的 Web 请求使用 credentials 模式。

CORS 不是认证机制。即使 Origin 被允许，API 仍需正常认证、Scope 和 CSRF 校验。

## 7. CSRF 与 XSS

### CSRF

Web 写操作继续走 EPIC-04 的 Session + CSRF 边界：

- Session Token 由 HttpOnly Cookie 携带；
- 前端提交 `x-csrf-token`；
- 服务端同时校验 Session、CSRF Token 与 Origin；
- Bearer API 调用不依赖 Cookie，因此不走 Cookie-CSRF 模型。

EPIC-17 的账号删除接口同样复用这一边界，不能因为它是“隐私接口”而绕开 CSRF。

### XSS

云端 API 不直接执行用户 HTML，并使用严格 CSP 降低错误内容类型下的执行风险。对于邮件等允许富文本的领域，服务端已有内容清洗依赖，前端仍必须使用结构化渲染或经过 sanitizer 的 HTML，禁止把未清洗外部文本直接传给危险 HTML 注入入口。

## 8. Token 与客户端安全

### Browser / PWA

长期认证 Token 不允许写入 `localStorage` / `sessionStorage`。浏览器使用 HttpOnly Session Cookie；JavaScript 只持有完成当前请求所需的非 HttpOnly CSRF 状态。

### Windows / Tauri

当前 `apps/desktop/src/services/cloudAuth.ts` 与 `apps/desktop/src-tauri/src/cloud_auth.rs` 的安全边界是：

- Access Token 只保存在运行时内存中；
- Refresh Token 通过 `cloudCredentialApi` 桥接到 **Windows Credential Manager**；
- Windows 原生层使用 `CredWriteW` / `CredReadW` / `CredDeleteW`，写入后的临时字节缓冲会清零；
- Web Storage 只保存服务 Origin 和随机 Device ID，这两项不是认证 Secret；
- 登出时删除 Windows Credential Manager 中的长期凭据并清空内存 Access Token；
- 非 Windows 构建若没有可用安全凭据存储，不退化为明文文件、Web Storage 或普通 SQLite 持久化。

因此长期 Refresh Token 的最终落点是操作系统凭据库，而不是“自制加密”文件。

## 9. App 权限最小化

授权继续使用 EPIC-04 Scope 模型。EPIC-17 新增隐私导出同样按 Scope 裁剪：

- 全量导出只包含当前 Principal 有读权限的模块；
- 分模块导出必须拥有对应 `<module>:read`；
- 账号删除必须拥有 `account:write`；
- 一个单领域 App 不会因为“导出”接口而获得其他领域数据。

桌面端 Tauri capability 同样采用最小权限：默认 capability 只对主窗口开放核心窗口几何、打开/保存对话框、更新器和进程重启等当前功能所需权限，不开放未使用的通用系统能力。

## 10. 数据库最小权限

生产环境新增约束：

```text
MIGRATION_ON_STARTUP=false
```

原因是生产运行时数据库角色不应拥有 `CREATE/ALTER/DROP` 等 schema 权限。推荐部署模型：

1. CI/CD 使用单独的 migration identity 执行 SQLx migration；
2. 应用运行时 identity 只获得所需 schema 的连接、查询和 DML 权限；
3. 数据库端口不暴露公网；
4. 密码/连接串通过部署 Secret 管理注入，不写入仓库；
5. 定期轮换生产数据库凭据。

## 11. Secret 管理与日志脱敏

禁止进入普通日志的内容包括：

- `Authorization`；
- `Cookie` / `Set-Cookie`；
- Password；
- Access / Refresh / CSRF Token；
- Token hash；
- API Key / Secret；
- 邮件账号 credential ciphertext；
- 完整邮件正文、通知原文、请求 raw body 等高敏内容。

云端 `security::redact_sensitive_json` 为结构化诊断元数据提供统一递归脱敏。桌面端已有 `sanitizeLogValue`，EPIC-17 进一步把 `bodyText`、`bodyHtml`、`rawBody`、`notificationContent` 等正文键加入强制脱敏规则；URL 日志继续移除 query string 和 fragment。

生产构建中的客户端 `debug` 事件会在日志创建前直接丢弃，不写控制台、本地日志存储或 Tauri 日志文件。生产日志应以请求 ID、错误码、模块、耗时、对象 ID 等最小必要元数据为主。

## 12. 公共设备与本地数据

公共设备模式沿用短 Session，并且不创建长期客户端凭据。退出后必须撤销服务端 Session；浏览器缓存策略为 `no-store`。

Windows SQLite 属于敏感的本地完整副本，应依赖 OS 用户 ACL、设备磁盘加密与应用权限隔离。当前 Epic 不宣称已经实现 SQLCipher 等数据库级透明加密；在没有成熟平台密钥管理前，不使用硬编码密钥或“自制加密”制造错误安全感。

## 13. 验证

EPIC-17 测试覆盖：

- 通用 API limiter 的配额、客户端隔离和窗口重置；
- 所有 API 响应安全 Header；
- production HSTS；
- production HTTP / wildcard CORS 拒绝；
- production runtime migration 拒绝；
- 匿名隐私 API 拒绝；
- 导出不包含 Password/Token/Credential Secret；
- 数据删除后用户、Session、Token 不再存在；
- 云端结构化日志 Secret 递归脱敏；
- 客户端认证 Secret 与完整敏感正文脱敏；
- 生产客户端 debug 日志关闭策略；
- Access/Refresh Token 不进入 Web Storage 的既有回归测试。

同时保留现有 Browser、Desktop、PostgreSQL、Clippy 等主线回归门禁。
