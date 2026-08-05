# LifeTrace EPIC-04：账号、认证与设备管理——Agent 具体实施方案

> 目标仓库：`zhouxingxing1279/LifeTrace`  
> 云端服务：`services/lifetrace-cloud`（Rust + Axum）  
> 公共契约：`crates/lifetrace-contracts`  
> 前置依赖：EPIC-02 契约；EPIC-03 PostgreSQL 持久化云端服务  
> 核心原则：**本地模式无需登录、服务端统一认证、每个 App 最小权限、凭据可立即撤销、Refresh Token 轮换、浏览器不保存长期 Token、敏感操作可审计。**

---

## 1. 最终目标

EPIC-04 完成后，以下客户端使用统一账号体系，但拥有独立 App 授权、设备安装、Session 和 Token：

```text
LifeTrace Desktop
LifeTrace Finance Android
LifeTrace Notes Android
LifeTrace English Android
LifeTrace Habits Android
LifeTrace Web
        ↓
统一账号系统
        ↓
App Grant + Scope + Device + Session + Token
        ↓
LifeTrace Cloud
```

用户必须能够：

- 初始化或注册账号；
- 登录、退出、修改密码、找回密码；
- 查看并撤销 Session、设备和 App 授权；
- 手机丢失后撤销相关设备与 Token；
- 单独撤销 Notes App 而不影响 Finance App；
- 在 Web 使用安全 Cookie Session；
- 未登录时继续完整使用 Windows 本地功能。

---

## 2. EPIC-03 前置门禁

Agent 开始前必须验证：

- [ ] `lifetrace-cloud` 已引入 SQLx PostgreSQL；
- [ ] `AppState` 持有真实 `PgPool`；
- [ ] 启动时实际执行 SQLx Migration；
- [ ] `/health/ready` 检查 PostgreSQL；
- [ ] PostgreSQL 不可用时 Ready 返回 503；
- [ ] 服务重启后实体、Change Log、幂等状态仍存在；
- [ ] Docker Compose 中 Cloud 实际连接 PostgreSQL；
- [ ] PostgreSQL 集成测试可运行。

任一项未满足时：

1. 生成 `docs/epic-04/precondition-report.md`；
2. 列出 EPIC-03 阻塞项；
3. 停止正式认证实现；
4. 禁止在内存 Store 中保存正式密码、Token 或 Session；
5. 禁止声称 EPIC-04 已开始实现。

---

## 3. 范围边界

### EPIC-04 必须实现

- 用户账号与管理员初始化；
- 可控注册模式；
- Argon2id 密码哈希；
- Native 登录、登出；
- 短期 Access Token；
- Refresh Token 轮换与重放检测；
- App Grant 和 Scope；
- 设备、Session 和 Token 撤销；
- 修改密码与忘记密码；
- Web Secure Cookie Session；
- CSRF 防护；
- 登录限流；
- 认证审计日志；
- Windows 云账号设置和安全凭据存储；
- Auth OpenAPI、生成类型和测试。

### 不属于本 Epic

- EPIC-05 Sync Outbox 和后台同步 Worker；
- 完整 Android 业务 App；
- 第三方社交登录、企业 SSO；
- 完整 MFA；
- 邮箱收件箱同步；
- 文件上传；
- AI 权限系统；
- 多租户组织和团队权限。

---

## 4. 核心架构

### 4.1 本地优先

```text
启动 Windows 应用
→ 直接读取本地 SQLite
→ 本地功能完整可用
→ 用户主动登录云账号
→ 才启用云端身份及后续同步
```

禁止把登录作为打开应用或读取本地数据的前置条件。

### 4.2 Token 模型

v1 使用：

```text
短期 Opaque Access Token
+
轮换 Opaque Refresh Token
+
服务端持久化 Session
```

默认不使用自包含 JWT，确保设备、App、Session 和 Token 可立即撤销。

### 4.3 Native 与 Web 分离

Native：

```text
Authorization: Bearer <access_token>
Refresh Token → OS 安全凭据存储
```

Web：

```text
__Host-lifetrace_session
Secure
HttpOnly
SameSite=Lax 或 Strict
Path=/
无 Domain
```

浏览器 JavaScript 不得获得长期 Refresh Token。

### 4.4 App 独立授权

同一用户拥有独立 Grant：

```text
user + lifetrace-desktop
user + lifetrace-finance-android
user + lifetrace-notes-android
user + lifetrace-english-android
user + lifetrace-habits-android
user + lifetrace-web
```

撤销一个 App Grant 只撤销该 App 的 Session 和 Token。

### 4.5 Scope 必须覆盖同步接口

通用 Push、Pull、Snapshot 必须执行：

```text
entityType
→ required domain scope
→ principal scopes
```

例如 Finance App Push `note.note` 必须拒绝，不能通过通用同步接口绕过权限。

---

## 5. 密码安全

- Argon2id；
- 每个密码使用随机 Salt；
- PHC 字符串保存算法和参数；
- 参数配置化并在部署硬件上基准测试；
- 登录成功时支持自动 Rehash；
- 最少 15 个 Unicode 字符；
- 至少允许 64 字符，建议允许 128；
- 允许空格和 Unicode；
- 不强制大小写、数字、符号组合；
- 不定期强制改密；
- 使用常见/泄露密码 Blocklist；
- 不 trim、不截断；
- 设置合理最大字节数防止哈希 DoS；
- 密码不得写入日志、错误信息或审计 Metadata。

---

## 6. Token 安全

Token 推荐格式：

```text
lt_at_<token_id>.<secret>
lt_rt_<token_id>.<secret>
lt_ws_<session_id>.<secret>
lt_pr_<token_id>.<secret>
```

要求：

- Secret 至少 32 字节 CSPRNG；
- 数据库只保存 Token Hash；
- 日志最多记录无敏感 Prefix；
- Token Parser 严格限制字符集和长度；
- Access Token 默认 15 分钟；
- Refresh Token 空闲过期默认 30 天；
- Refresh Token 绝对过期默认 90 天；
- Web Session 空闲 12 小时、绝对 7 天；
- 公共设备 Session 最大 8 小时；
- Password Reset Token 默认 30 分钟；
- 生命周期必须配置化，客户端以响应为准。

随机 Token 可使用 SHA-256 或带服务端 Pepper 的 HMAC-SHA-256 计算 Hash；Argon2 只用于用户密码。

---

## 7. 公共认证契约

在 `crates/lifetrace-contracts/src/auth/v1/` 增加：

```text
account.rs
login.rs
token.rs
password.rs
session.rs
device.rs
app_grant.rs
scope.rs
web.rs
```

基础类型：

```rust
pub struct AuthSessionId(String);
pub struct AppGrantId(String);
pub struct AppInstallationId(String);
pub struct TokenFamilyId(String);
pub struct Scope(String);
```

核心 DTO：

- `AuthUserV1`
- `LoginRequestV1`
- `TokenResponseV1`
- `RefreshRequestV1`
- `WebLoginResponseV1`
- `DeviceInstallationV1`
- `AppGrantV1`
- Session、Password Change、Forgot、Reset 请求/响应

服务端签发 Scope 时执行：

```text
requested scopes
∩ app policy allowed scopes
∩ user app grant scopes
= issued scopes
```

不得在响应中返回：

- `password_hash`；
- Token Hash；
- Reset Token Hash；
- 内部安全标记；
- 完整登录失败记录。

---

## 8. Scope 与 App Policy

初始 Scope：

```text
account:read account:write
devices:read devices:write
sessions:read sessions:write
sync:read sync:write
finance:read finance:write
notes:read notes:write
files:read files:write
english:read english:write
habits:read habits:write
reviews:read reviews:write
workouts:read workouts:write
```

App 最大权限：

- Desktop：所有 LifeTrace 业务域；
- Finance Android：account/device read、sync、finance；
- Notes Android：account/device read、sync、notes、files；
- English Android：account/device read、sync、english；
- Habits Android：account/device read、sync、habits、reviews；
- Web：用户授权的业务域，高风险账号/设备写权限需明确确认。

建立唯一映射：

```text
finance.* → finance:read/write
note.* → notes:read/write
english.* → english:read/write
habit.* → habits:read/write
review.daily → reviews:read/write
workout.* → workouts:read/write
file.metadata → files:read/write
```

Push 使用 Write Scope；Pull/Snapshot 使用 Read Scope；未知 Entity Type 不得默认放行。

---

## 9. PostgreSQL Migration

只追加 Migration，禁止修改 EPIC-03 已发布文件。

建议：

```text
0007_auth_users.sql
0008_auth_apps_scopes.sql
0009_auth_devices_sessions.sql
0010_auth_tokens.sql
0011_auth_password_recovery.sql
0012_auth_rate_limit_audit.sql
0013_auth_indexes_constraints.sql
```

### 扩展 cloud_users

增加：

```text
email
email_normalized
display_name
password_hash
password_version
email_verified_at
password_changed_at
disabled_at
registration_source
failed_login_count
locked_until
auth_state
```

`email_normalized` 唯一；`auth_state` 推荐：

```text
pending
active
password_reset_required
disabled
```

### auth_app_grants

保存：

```text
id user_id app_id scopes status granted_at revoked_at updated_at
```

代码内 Policy Registry 决定 App 最大权限，数据库 Grant 只能缩小，不能扩大。

### cloud_devices / App Installation

扩展：

```text
device_group_id
device_name
last_sync_at
last_login_at
last_login_ip
last_user_agent
revoked_at
revoked_reason
```

### auth_sessions

保存：

```text
id user_id device_id app_id scopes session_type status
created_at last_seen_at idle_expires_at absolute_expires_at
revoked_at revoked_reason login_ip last_ip user_agent public_device
```

### Token 表

```text
auth_access_tokens
auth_refresh_tokens
auth_web_sessions
```

Refresh Token 必须包含：

```text
family_id
parent_token_id
replaced_by_token_id
used_at
revoked_at
reuse_detected_at
```

### Password Recovery 与审计

```text
auth_password_reset_tokens
auth_login_attempts
auth_audit_log
```

审计 Metadata 不得存密码、Token、Cookie Secret、Reset Token 或完整请求 Body。

---

## 10. AuthProvider 改造

当前同步 `AuthProvider` 改为异步数据库实现：

```rust
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(
        &self,
        credential: AuthCredential<'_>,
    ) -> Result<AuthenticatedPrincipal, ApiError>;
}
```

新的 Principal：

```rust
pub struct AuthenticatedPrincipal {
    pub user_id: UserId,
    pub session_id: AuthSessionId,
    pub device_id: AppInstallationId,
    pub app_id: AppId,
    pub scopes: ScopeSet,
    pub auth_method: AuthMethod,
}
```

`DatabaseAuthProvider` 验证：

```text
Token Hash
→ Access Token 状态和过期
→ Session 状态和过期
→ User 状态
→ Device 状态
→ App Grant 状态
→ 当前 Scope 交集
```

Handler 不得直接查询 Token 表。

---

## 11. 登录与 Refresh 流程

### Native Login

```text
规范化 Email
→ 检查账号/IP 限流
→ 查询用户
→ 虚拟 Hash 防止账号枚举时序差异
→ Argon2id Verify
→ 检查用户状态
→ 校验 AppId
→ 获取/创建 App Grant
→ 计算 issued scopes
→ 注册/更新 Device Installation
→ 创建 Session
→ 创建 Access Token
→ 创建 Refresh Token Family
→ Audit
→ 返回 TokenResponse
```

公开错误统一为“邮箱或密码错误”，内部审计记录真实原因。

### Refresh Rotation

必须在单个 PostgreSQL 事务中：

```text
SELECT refresh token FOR UPDATE
→ 校验 Hash/Session/App/Device/User
→ 检查未使用、未撤销、未过期
→ 标记旧 Token used_at
→ 创建新 Refresh Token
→ 关联 replaced_by_token_id
→ 创建新 Access Token
→ 更新 Session
→ Audit
→ COMMIT
```

### Refresh 重放

提交已经使用或已经被替换的 Refresh Token：

```text
设置 reuse_detected_at
→ 撤销同 family_id 全部 Refresh Token
→ 撤销 Session
→ 撤销 Session Access Token
→ 写安全 Audit
→ 返回 AUTH_REFRESH_TOKEN_REUSED
```

客户端必须使用 single-flight，禁止并发 Refresh。

---

## 12. 注册与管理员初始化

配置：

```text
AUTH_REGISTRATION_MODE=disabled|invite|open
```

默认 `disabled`。

提供 CLI：

```text
lifetrace-cloud admin bootstrap-user
```

要求：

- 直接连接数据库；
- 不通过公开 HTTP；
- 密码从 stdin/交互读取，不进入 Shell History；
- 已有 Active 用户时默认拒绝重复 bootstrap；
- 额外用户必须显式指定参数。

Invite 模式令牌必须高熵、只存 Hash、一次性、有过期时间，并可限定邮箱。

---

## 13. Logout、设备和 App 撤销

### 当前 Session Logout

- 撤销 Session；
- 撤销全部 Access/Refresh Token；
- Web 清除 Cookie；
- 写 Audit。

### Logout All

默认撤销当前用户所有 Session，包括当前 Session。

### App Grant 撤销

```text
DELETE /api/v1/auth/apps/{appId}/grant
```

仅撤销该 App 的 Grant、Session 和 Token，不影响其他 App。

### 设备撤销

```text
POST /api/v1/auth/devices/{deviceId}/revoke
```

设备状态改为 revoked，并撤销该安装全部 Session 和 Token。

### 丢失手机

支持 Device Group 批量撤销；若无法提供稳定 Device Group ID，设备页面必须支持一次选择多个安装撤销。

---

## 14. 密码修改与找回

### 修改密码

```text
认证当前 Session
→ 重新验证当前密码
→ 校验新密码策略
→ Argon2id Hash
→ 更新 password_hash/version/changed_at
→ 撤销所有 Session
→ Audit
```

默认所有设备退出。

### Forgot Password

无论邮箱是否存在，都返回相同 `202 Accepted`。

Password Reset Token：

- 高熵；
- 数据库只存 Hash；
- 一次性；
- 默认 30 分钟；
- 新请求可撤销旧 Token；
- 独立限流。

定义 `PasswordResetNotifier`：测试、开发 Console、生产 SMTP/Notifier。生产环境不得返回 Reset Token，也不得启用开发 Console Notifier。

Reset 成功后必须撤销全部 Session 和 Token。

---

## 15. 登录限流

至少按两类 Key：

```text
normalized email hash
IP address
```

推荐初始策略：

```text
账号：5 次失败 / 15 分钟
IP：30 次失败 / 15 分钟
临时锁定：15 分钟，连续触发逐步增加
```

所有参数配置化；限流状态存 PostgreSQL，保证多实例一致；成功登录后重置账号连续失败计数；不得永久锁死账号。

只信任配置中的受控反向代理；未配置可信代理时使用 Socket Peer IP，禁止无条件信任 `X-Forwarded-For`。

---

## 16. Web Session 与 CSRF

Web 登录：

- 不返回 Access/Refresh Token；
- 创建服务端 Web Session；
- `Set-Cookie` 写入 `__Host-lifetrace_session`；
- 响应返回 CSRF Token。

所有 `POST/PUT/PATCH/DELETE`：

- 验证 `X-CSRF-Token`；
- Hash 后常量时间比较；
- 同时验证 Origin/Referer；
- 不只依赖 SameSite。

Session 必须在以下情况轮换或撤销：

- 登录成功；
- 密码修改/重置；
- Scope 变化；
- App Grant 变化；
- 可疑活动；
- 手工 Session Rotation。

公共设备模式：无长期 Refresh，最大 8 小时，浏览器关闭失效，禁止“记住我”，Logout 清 Cookie。

---

## 17. Windows 安全存储与 UI

### 安全存储

Refresh Token 必须进入 Windows Credential Manager 或对应系统 Keyring。

禁止保存到：

```text
SQLite
JSON 设置
localStorage
日志
明文配置文件
Tauri 前端持久化状态
```

Access Token 只在内存，应用退出即丢弃。

### Auth Store

职责：

```text
access token memory
refresh token secure storage
single-flight refresh
401 retry once
logout cleanup
account state
```

遇到 Token 重放、Session 撤销或 Device 撤销时，清除本地凭据并回到未登录状态。

### 云账号设置页

未登录时明确显示：本地模式运行，本地数据不受影响。

登录后显示：账号、当前设备、当前 App、Session、最近在线、设备列表、App Grant、退出当前设备和退出全部设备。

---

## 18. API

### Public Native

```text
POST /api/v1/auth/register
POST /api/v1/auth/login
POST /api/v1/auth/refresh
POST /api/v1/auth/password/forgot
POST /api/v1/auth/password/reset
GET  /api/v1/auth/capabilities
```

### Authenticated

```text
GET  /api/v1/auth/me
POST /api/v1/auth/logout
POST /api/v1/auth/logout-all
POST /api/v1/auth/password/change
GET    /api/v1/auth/sessions
DELETE /api/v1/auth/sessions/{sessionId}
GET    /api/v1/auth/devices
PATCH  /api/v1/auth/devices/{deviceId}
POST   /api/v1/auth/devices/{deviceId}/revoke
POST   /api/v1/auth/device-groups/{deviceGroupId}/revoke
GET    /api/v1/auth/apps
PATCH  /api/v1/auth/apps/{appId}/grant
DELETE /api/v1/auth/apps/{appId}/grant
```

### Web

```text
POST /api/v1/web/session/login
GET  /api/v1/web/session
POST /api/v1/web/session/rotate
POST /api/v1/web/session/logout
GET  /api/v1/web/csrf
```

---

## 19. 稳定错误码

增加：

```text
LIFETRACE_AUTH_REQUIRED
LIFETRACE_AUTH_INVALID
LIFETRACE_AUTH_PASSWORD_INVALID
LIFETRACE_AUTH_USER_DISABLED
LIFETRACE_AUTH_USER_LOCKED
LIFETRACE_AUTH_ACCESS_TOKEN_EXPIRED
LIFETRACE_AUTH_REFRESH_TOKEN_EXPIRED
LIFETRACE_AUTH_REFRESH_TOKEN_REUSED
LIFETRACE_AUTH_SESSION_REVOKED
LIFETRACE_AUTH_DEVICE_REVOKED
LIFETRACE_AUTH_APP_REVOKED
LIFETRACE_AUTH_SCOPE_DENIED
LIFETRACE_AUTH_PASSWORD_POLICY_FAILED
LIFETRACE_AUTH_PASSWORD_RESET_INVALID
LIFETRACE_AUTH_PASSWORD_RESET_EXPIRED
LIFETRACE_AUTH_RATE_LIMITED
LIFETRACE_AUTH_CSRF_INVALID
LIFETRACE_AUTH_REGISTRATION_DISABLED
LIFETRACE_AUTH_INVITE_INVALID
LIFETRACE_APP_ID_UNSUPPORTED
```

公开 Login 错误不得区分 User Not Found 和 Password Wrong。

---

## 20. 配置

至少支持：

```text
AUTH_REGISTRATION_MODE
AUTH_ACCESS_TOKEN_TTL_SECONDS
AUTH_REFRESH_IDLE_TTL_SECONDS
AUTH_REFRESH_ABSOLUTE_TTL_SECONDS
AUTH_WEB_IDLE_TTL_SECONDS
AUTH_WEB_ABSOLUTE_TTL_SECONDS
AUTH_PUBLIC_DEVICE_TTL_SECONDS
AUTH_ARGON2_MEMORY_KIB
AUTH_ARGON2_ITERATIONS
AUTH_ARGON2_PARALLELISM
AUTH_PASSWORD_MIN_LENGTH
AUTH_PASSWORD_MAX_BYTES
AUTH_PASSWORD_BLOCKLIST_PATH
AUTH_PASSWORD_PEPPER
AUTH_TOKEN_HASH_PEPPER
AUTH_RESET_TOKEN_TTL_SECONDS
AUTH_LOGIN_ACCOUNT_LIMIT
AUTH_LOGIN_IP_LIMIT
AUTH_LOGIN_WINDOW_SECONDS
AUTH_LOCKOUT_SECONDS
AUTH_COOKIE_NAME
AUTH_COOKIE_SAME_SITE
AUTH_COOKIE_SECURE
AUTH_TRUSTED_PROXY_CIDRS
AUTH_RESET_NOTIFIER
SMTP_HOST SMTP_PORT SMTP_USERNAME SMTP_PASSWORD SMTP_FROM
PUBLIC_WEB_BASE_URL
```

生产环境必须 Fail Closed：Cookie Secure=true、公开 URL 使用 HTTPS、开发 Auth/Notifier 禁用、开放注册显式配置、所有 Pepper/Secret 不得使用默认值。

---

## 21. 测试

### 单元测试

- Email 规范化；
- Password Policy；
- Argon2id Hash/Verify/Rehash；
- Token Generate/Parse/Hash；
- Scope Intersection 和 App Policy；
- Entity Type Scope；
- Cookie Builder；
- CSRF；
- 可信代理 IP；
- Token 日志脱敏。

### PostgreSQL 集成测试

- User、App Grant、Device、Session；
- Access Token 查询；
- Refresh Rotation；
- Refresh 重放撤销 Family；
- Session、Device、App 撤销；
- Password Reset 一次性；
- Login Rate Limit；
- Audit；
- Migration 从空库执行并兼容 EPIC-03 数据。

### API 测试

- Registration Disabled/Open/Invite；
- 正确和错误登录；
- 用户枚举防护；
- Access/Refresh 过期与撤销；
- 并发 Refresh；
- Finance App 访问 Notes 被拒绝；
- Sync Push/Pull/Snapshot 越权被拒绝；
- Device、Session、App 撤销立即生效；
- Change Password；
- Forgot/Reset；
- Web Cookie 属性；
- CSRF、Origin、Session Rotation、Public Device。

### Windows 测试

- 未登录本地功能可用；
- Refresh Token 进入安全存储；
- SQLite/JSON 不出现 Refresh Token；
- Access Token 不持久化；
- Single-flight Refresh；
- 401 只重试一次；
- Logout 清理凭据；
- Device Revoked 后退出。

---

## 22. 文档和生成物

生成：

```text
docs/epic-04/current-auth-audit.md
docs/epic-04/architecture.md
docs/epic-04/database-schema.md
docs/epic-04/auth-api.md
docs/epic-04/app-scope-policy.md
docs/epic-04/native-token-storage.md
docs/epic-04/web-session-security.md
docs/epic-04/password-recovery.md
docs/epic-04/threat-model.md
docs/epic-04/operations.md
docs/epic-04/validation-report.md
docs/epic-04/completion-report.md
```

Contract Exporter 墺加：

```text
contracts/openapi/lifetrace-auth-v1.json
contracts/json-schema/auth-*.schema.json
contracts/typescript/lifetrace-auth.generated.ts
```

执行 `contracts:generate` 和 `contracts:check`。

---

## 23. 分阶段实施

```text
阶段 0：前置审计
阶段 1：Auth Contract
阶段 2：PostgreSQL Migration
阶段 3：Password/Token/Cookie/CSRF 基础
阶段 4：Bootstrap 与 Registration
阶段 5：Native Login 与 Refresh Rotation
阶段 6：App Scope 与 Sync Authorization
阶段 7：Device 与 Session 管理
阶段 8：Password Change 与 Recovery
阶段 9：Web Session
阶段 10：Windows Integration
阶段 11：Security 与 Acceptance
```

每阶段单独提交并运行测试，不得一次完成全部代码后再测试。

推荐提交：

```text
docs(auth): audit epic-04 prerequisites
feat(contracts): add auth protocol v1
feat(auth-db): add user app device session schema
feat(auth): add password and token primitives
feat(auth): add account bootstrap and registration
feat(auth): implement native login and refresh rotation
feat(authz): enforce app scopes across sync and crud
feat(auth): add device and session management
feat(auth): add password recovery
feat(web-auth): add secure cookie sessions and csrf
feat(desktop): add optional cloud account login
test(auth): add postgres concurrency and security tests
docs(auth): complete epic-04 documentation
```

---

## 24. Definition of Done

- [ ] EPIC-03 PostgreSQL 持久化与 Docker Smoke Test 通过；
- [ ] 管理员安全初始化和可控注册；
- [ ] Login、Logout、Logout All；
- [ ] Argon2id、Blocklist、Rate Limit；
- [ ] Access Token 短期有效且数据库只存 Hash；
- [ ] Refresh 每次轮换；
- [ ] 重放检测撤销 Token Family；
- [ ] 并发 Refresh 测试通过；
- [ ] 所有官方 App 有 Scope Policy；
- [ ] App Grant 可独立撤销；
- [ ] Finance App 无法访问 Notes；
- [ ] Sync API 无法绕过 Scope；
- [ ] 设备注册、命名、最近登录/同步和撤销；
- [ ] Session 列表和撤销；
- [ ] 修改密码与一次性 Password Reset；
- [ ] Web Secure/HttpOnly/SameSite/`__Host-` Cookie；
- [ ] CSRF 和 Session Rotation；
- [ ] Web 不把 Token 写入 localStorage/sessionStorage；
- [ ] Windows 本地模式无需登录；
- [ ] Refresh Token 使用 Windows Credential Manager；
- [ ] Access Token 只在内存；
- [ ] Audit Log 与 Threat Model；
- [ ] Production Config Fail Closed；
- [ ] PostgreSQL 集成、并发、安全和 Docker 测试通过；
- [ ] OpenAPI/Schema/TypeScript 生成物最新；
- [ ] Completion Report 完整。

---

## 25. Agent 第一轮提示词

```text
请完整阅读：

- docs/epic-04/LifeTrace_EPIC04_Agent_Implementation_Plan.md
- docs/epic-02/**
- docs/epic-03/**
- crates/lifetrace-contracts/**
- services/lifetrace-cloud/**
- 当前 PostgreSQL 和 Docker 配置

本轮只执行 EPIC-04 阶段 0：前置审计。

生成：

docs/epic-04/current-auth-audit.md

必须验证：

- PostgreSQL 是否真正接线；
- AppState 是否使用 PgPool；
- Ready 是否检查数据库；
- 服务重启后数据是否持久化；
- AuthProvider 当前实现；
- cloud_users/cloud_devices 当前字段；
- AppId/DeviceId/Scope/ErrorCode 现状；
- 客户端是否已有登录功能；
- 当前 Token 存储风险；
- 方案调整项；
- 阻塞项；
- 下一阶段文件清单。

如果 EPIC-03 PostgreSQL 仍未完成：

生成 docs/epic-04/precondition-report.md 后停止。

本轮不得：

- 实现内存账号系统；
- 保存真实密码；
- 创建正式 Token；
- 修改登录 UI；
- 提前执行阶段 1；
- 声称 EPIC-04 已开始开发。

完成审计后停止。
```
