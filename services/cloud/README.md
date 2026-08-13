# LifeTrace Cloud

LifeTrace 独立云端后端服务（Rust + Axum + PostgreSQL）。它与桌面应用位于同一个 Monorepo，但可以独立构建、测试和部署。

## 目录

- 云端源码：`services/cloud/src/`
- 数据库迁移：`services/cloud/migrations/`
- 云端测试：`services/cloud/tests/`
- 部署文件：`deploy/cloud/`
- 共享协议：`crates/lifetrace-contracts/`、`contracts/`

## 端点

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/health/live` | 否 | 存活 |
| GET | `/health/ready` | 否 | 就绪 |
| GET | `/api/v1/meta/version` | 否 | 版本 |
| GET | `/api/v1/sync/capabilities` | 否 | 能力协商 |
| POST | `/api/v1/sync/push` | Bearer / Web Session | 批量提交 |
| POST | `/api/v1/sync/pull` | Bearer / Web Session | 拉取变更 |
| POST | `/api/v1/sync/snapshot` | Bearer / Web Session | 全量快照 |
| GET | `/api/v1/privacy/export` | Bearer / Web Session | 导出当前授权范围内的全部用户数据 |
| GET | `/api/v1/privacy/export/{module}` | Bearer / Web Session | 分模块导出 |
| GET | `/api/v1/privacy/policy` | Bearer / Web Session | 查看数据保留策略 |
| DELETE | `/api/v1/privacy/account` | Bearer / Web Session + CSRF | 注销账号并清理云端数据 |
| GET | `/api/v1/integrations/beecount/status` | Bearer / Web Session + `finance:read` | BeeCount 只读适配器状态 |
| GET | `/api/v1/integrations/beecount/ledgers` | Bearer / Web Session + `finance:read` | BeeCount 账本列表 |
| GET | `/api/v1/integrations/beecount/ledgers/{ledger_id}/snapshot` | Bearer / Web Session + `finance:read` | 规范化交易、账户、分类、标签与预算快照 |

## 运行

从仓库根目录：

```powershell
npm run dev:cloud
```

或直接：

```powershell
cargo run --manifest-path services/cloud/Cargo.toml
```

本地 PostgreSQL / Docker 环境见 `deploy/cloud/` 和 `scripts/cloud/`。

## 测试

```powershell
npm run test:cloud
```

或：

```powershell
cargo test --manifest-path services/cloud/Cargo.toml
```

## 生产安全配置

EPIC-17 在现有 EPIC-04 认证基础上增加统一 CSP、安全响应头、HSTS、隐私导出/删除与更严格的生产配置检查。

生产部署要求：

- TLS 在可信反向代理、Ingress 或负载均衡器终止，外部访问只使用 HTTPS；
- `PUBLIC_WEB_BASE_URL` 必须是 HTTPS；
- Session Cookie 必须开启 Secure；
- `CORS_ALLOWED_ORIGINS` 只填写显式 HTTPS Origin，禁止 `*`、`null` 和 HTTP Origin；
- `MIGRATION_ON_STARTUP=false`；
- migration 使用独立高权限数据库身份，运行时数据库身份只授予所需 DML 权限；
- Secret 通过部署系统注入，不提交到仓库，不输出到日志；
- 数据库端口不暴露公网。

服务端统一返回 `nosniff`、`no-referrer`、`DENY` frame policy、严格 CSP、`no-store` 等响应头；production 额外返回 HSTS。

## 数据删除说明

`DELETE /api/v1/privacy/account` 会验证认证/Scope，Web Session 调用还会执行 CSRF 校验。当前 PostgreSQL schema 以 `cloud_users` 为用户所有权根，账号删除前先撤销活动 Session，再在事务中删除用户根，由外键级联清理设备、同步数据、Session/Token 和邮件数据。

BeeCount 兼容附件保存在 PostgreSQL `cloud_file_blobs`，随 `cloud_users` 外键级联删除；邮件附件
的外部 `storage_ref` 仍没有通用对象存储清理 Provider。如果邮件附件存在非空
`storage_ref`，接口会返回错误而不是虚报账号及外部文件已经删除。引入 S3/OSS 等对象存储
后，必须先补齐幂等对象删除再放开该门禁。

完整设计见：

- `docs/epic-17/security-architecture.md`
- `docs/epic-17/data-lifecycle.md`

## 部署

生产镜像使用 `services/cloud/Dockerfile`。云端服务不依赖 `apps/desktop`，部署时只需要云端源码、共享 Rust crates 与部署配置。

生产 Compose 还可以在同一台服务器上启动 BeeCount Cloud 兼容服务，使无需
重新构建的 BeeCount iOS 客户端通过独立 HTTPS 域名连接。该服务与 LifeTrace
PostgreSQL 数据隔离，部署、验证和备份说明见
`docs/beecount-cloud-integration/deployment.md`。

启用 `BEECOUNT_ADAPTER_ENABLED=true` 后，适配器只允许
`BEECOUNT_ADAPTER_LIFETRACE_USER_ID` 指定的 LifeTrace 用户读取数据，并只
连接同一 Compose 网络中的 `http://beecount-cloud:8080/`（测试也可使用本机
回环地址）。服务账号密码只从部署环境读取；浏览器通过 LifeTrace Session
访问 `/finance/beecount`，不会接触 BeeCount 凭据。

兼容层附件上传上限由 `BEECOUNT_ATTACHMENT_MAX_UPLOAD_BYTES` 控制，默认 64 MiB。第四阶段的
附件与 WebSocket 内部接口、存储边界和切流门禁见
`docs/beecount-cloud-integration/phase-4-execution-report.md`。

第五阶段继续复用 `cloud_users`、`cloud_devices` 和 PostgreSQL 实体日志，补齐 Profile/头像、
设备撤销及共享账本 Owner/Editor、邀请、转让和共享资源快照。共享关系只保存权限元数据，
Editor 写入不会生成第二份财务实体。接口和剩余生产门禁见
`docs/beecount-cloud-integration/phase-5-execution-report.md`。
