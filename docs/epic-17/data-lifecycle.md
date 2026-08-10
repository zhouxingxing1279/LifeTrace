# EPIC-17 数据生命周期

> 状态：已实现  
> 更新日期：2026-08-10  
> 目标：让用户能够读取、迁移和删除自己的云端数据，并明确哪些数据何时保留、何时删除、哪些删除需要外部系统配合。

## 1. 生命周期原则

LifeTrace 采用以下规则：

1. 用户拥有的数据必须可导出，不以认证 Secret 作为“可迁移数据”。
2. 导出接口必须服从现有认证和 App Scope，不能成为越权旁路。
3. 账号注销必须先撤销会话，再删除用户所有权根数据。
4. 任何无法确认已经清理的外部对象都不能返回“删除成功”。
5. 备份删除与在线数据库删除是两个不同层次，必须分别说明。
6. 日志、导入原文、通知/邮件原文只保留实现业务或安全目的所需的最小集合。

## 2. 隐私 API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/api/v1/privacy/export` | 导出当前 Principal 有权读取的全部模块 |
| `GET` | `/api/v1/privacy/export/{module}` | 导出单一模块 |
| `GET` | `/api/v1/privacy/policy` | 返回当前数据保留策略摘要 |
| `DELETE` | `/api/v1/privacy/account` | 注销账号并删除云端用户根数据 |

Web Session 调用删除接口时必须通过 CSRF；原生客户端可使用 Bearer Token。

## 3. 导出格式

第一版格式标识：

```text
lifetrace-privacy-export-v1
```

顶层结构：

```json
{
  "format": "lifetrace-privacy-export-v1",
  "exportedAt": "...",
  "userId": "...",
  "requestedModule": null,
  "sections": {}
}
```

`sections` 按领域组织，便于人工阅读、归档和以后导入器做迁移。

## 4. 可导出模块

当前 API 支持：

- `account`
- `devices`
- `sessions`
- `finance`
- `notes`
- `files`
- `english`
- `habits`
- `reviews`
- `workouts`
- `execution`
- `mail`

业务实体从共享 Registry 中读取，仅导出 `UserOwned` 实体。数据库专有的账号、设备、Session、邮件数据通过显式安全查询补充。

全量导出不是“绕过权限的超级接口”：返回模块会继续按当前 Principal 的 Scope 过滤。例如只拥有 `finance:read` 的独立应用不能通过隐私导出读取 Notes。

## 5. 永不导出的认证 Secret

以下字段不是用户可迁移业务数据，禁止进入导出包：

- password hash；
- Access Token / Refresh Token 原文；
- Token hash；
- Web Session 原始 Token；
- CSRF Secret；
- password/token pepper；
- API Secret；
- 邮件账号 `credential_ciphertext` / `credential_nonce`；
- 其他可直接恢复认证能力的内部密钥材料。

账号导出采用字段 allowlist，而不是对 `cloud_users` 直接 `to_jsonb(*)`，防止以后新增认证列时被无意导出。

## 6. 账号删除流程

当前 PostgreSQL 数据模型以 `cloud_users` 作为用户数据所有权根。删除顺序：

```text
authenticated principal
        ↓
Bearer auth OR Web Session + CSRF
        ↓
require account:write
        ↓
检查是否存在无法由当前服务清理的外部对象
        ↓
BEGIN transaction
        ↓
revoke active auth_sessions
        ↓
DELETE cloud_users
        ↓
FK ON DELETE CASCADE 清理用户所属数据
        ↓
COMMIT
        ↓
清除浏览器 Session Cookie
```

如果关键步骤失败，事务不提交，接口不能返回成功。

## 7. 级联删除范围

当前 schema 的用户所有权关系确保删除 `cloud_users` 后至少覆盖：

- `cloud_devices`；
- `sync_entities` 及其同步变更、snapshot、processed-change 元数据；
- `auth_sessions`；
- Access / Refresh Token 等 Session 子资源；
- Web Session、设备授权等认证关联数据；
- `mail_accounts`；
- 邮件、收件人、附件、草稿、发送队列等邮件子资源。

测试不只检查 HTTP 204，还会查询 PostgreSQL 验证用户、Session 和 Token 已经不存在。

## 8. 文件对象删除

数据库记录删除不等于对象存储中的字节已经删除。

当前云服务尚未配置通用对象存储 cleanup provider，因此采取严格边界：

- 对纯数据库/本地元数据，可以正常随账号级联删除；
- 如果 `mail_attachments.storage_ref` 非空，说明存在外部对象引用；
- 当前服务没有能力确认该对象字节已被删除时，账号注销返回 `503`，不会声称成功；
- 后续引入 S3/OSS/其他对象存储时，应先实现幂等对象删除，再允许该检查通过。

未来对象删除 Provider 必须满足：

- 删除接口幂等；
- 按用户 ownership 校验对象；
- 失败可重试；
- 对象删除完成后才能提交账号最终删除；
- 记录对象 ID/结果，不记录对象正文或认证 Secret。

## 9. Session 与 Token

账号删除进入事务后，活动 Session 先标记 revoked，再删除用户根。这样即使后续代码演进为异步清理，也有明确的“先停止访问，再清数据”语义。

在当前 schema 中，最终用户根删除会级联删除 Session 及其 Access/Refresh Token 记录。删除完成后旧 Token 不应再通过认证。

设备丢失但不注销账号时，仍使用 EPIC-04 的单设备/Session 撤销能力，不必删除全部账号数据。

## 10. 通知与邮件原文

LifeTrace 邮件聚合属于用户显式启用的业务数据。当前策略：

- 在线账号存在期间，可保留实现邮件聚合所需的邮件正文和结构化字段；
- 账号注销时随账号删除；
- 诊断日志禁止复制完整邮件正文；
- 如果未来只需要摘要，应允许配置为“结构化摘要保留、原文更短期限”；
- 推送通知不应为了历史分析而永久保存第三方通知原文，除非该模块有明确用户可见功能和保留说明。

## 11. 导入文件

当前财务/数据导入的原始文件默认属于设备本地处理资产，不因同步业务实体而自动上传云端。

建议执行策略：

1. 原始导入文件进入临时任务目录；
2. 完成解析、去重和用户确认；
3. 业务结构化数据写入本地/同步层；
4. 原始文件按用户设置清理或保留在用户明确选择的位置；
5. 临时文件不进入普通备份和日志。

如果未来某模块要把原始文件上传云端，必须显式加入导出/删除和对象 cleanup 契约，不能只新增 upload。

## 12. 备份保留

“在线删除成功”意味着活动数据库和受控在线对象已经完成逻辑/物理删除流程，但不承诺已经从所有历史备份块中瞬时擦除。

部署必须定义明确的最大备份保留窗口，并遵循：

- 备份加密并限制访问；
- 过期备份自动销毁；
- 不为被注销用户单独延长历史备份生命周期；
- 如果从删除前备份恢复，必须重新执行已记录的删除/注销动作，避免被删除账号复活；
- 备份系统不得成为可直接查询的“影子生产库”。

仓库不硬编码某个天数，因为实际窗口属于部署策略；生产运维文档必须给出真实值。

## 13. 日志与审计数据

安全审计日志只保留调查所需的最小字段，例如：

- request ID；
- user/device/app ID；
- 事件类型；
- 时间；
- 结果/错误码；
- 必要的来源 IP 元数据。

不得记录 Password、Token、Cookie、邮件账号 Secret 或完整用户正文。账号删除后的安全审计如果因安全/合规目的需要短期保留，应与业务数据隔离，并避免保存可以重建用户内容的字段。

## 14. 运维删除检查清单

当账号删除失败时按以下顺序检查：

- 是否通过了正确认证与 `account:write`；
- Web 调用是否携带有效 CSRF/Origin；
- PostgreSQL 是否可用；
- 是否存在非空 `storage_ref` 且没有对象 cleanup provider；
- FK/schema 是否被未迁移的部署破坏；
- 是否有事务错误。

不得通过手工返回 204、忽略对象删除错误或仅把用户标记 inactive 来绕过失败。

## 15. 验收标准对应关系

- **可导出**：全量 + 分模块 JSON API；
- **可读**：稳定顶层格式、领域 sections；
- **不泄密**：字段 allowlist + Secret 排除测试；
- **匿名拒绝**：privacy 路由认证测试；
- **注销清理**：真实 PostgreSQL 集成测试验证 user/session/token；
- **Session 撤销**：删除前 revoke，再级联清理；
- **对象删除正确性**：没有 cleanup provider 时 fail-closed；
- **保留期可解释**：在线数据、邮件/通知原文、导入文件、备份分别文档化。
