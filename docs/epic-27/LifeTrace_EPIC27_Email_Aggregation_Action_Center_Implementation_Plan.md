# LifeTrace EPIC-27 邮件聚合与行动中心详细执行计划

> EPIC：EPIC-27「邮件聚合与行动中心」  
> 状态：Ready for Implementation  
> 更新日期：2026-08-09  
> 当前实施策略：**先完成非 AI 邮件基础能力，AI 能力整体后置**  
> 目标仓库：`zhouxingxing1279/LifeTrace`  
> 目标目录：`docs/epic-27/`  
> 依据：`docs/LifeTrace_Complete_Roadmap_v2.md` 中 EPIC-27，以及当前 LifeTrace `apps/desktop` + `services/cloud` 架构。

---

## 1. 当前阶段目标

EPIC-27 当前阶段先解决“邮件可靠接入和行动闭环”的基础问题，不在这一轮开发任何 AI 能力。

本轮完成后，LifeTrace 应能够：

1. 接入 QQ、163、126、yeah.net 以及通用 IMAP/SMTP 邮箱。
2. 安全保存邮箱授权凭据。
3. 首次同步最近 30 天邮件。
4. 基于 UID / UIDVALIDITY 做可靠增量同步。
5. 支持 IMAP IDLE，并在不可用时自动退回 2～5 分钟轮询。
6. 正确解析邮件正文、附件元数据和邮件线程。
7. 提供邮件列表、线程阅读、搜索、已读、归档等基础功能。
8. 用户可以**手动**把邮件转换为任务、事件、笔记或等待事项。
9. 用户可以手动撰写回复，并通过 SMTP 发送。
10. 邮件发送、同步、失败重试、日志和监控具备完整可观测性。

当前闭环：

```text
QQ / 163 / 126 / yeah.net / 通用 IMAP 邮箱
                    ↓
              邮箱账号连接
                    ↓
            云端 Mail Worker
         ↓                    ↓
     IMAP IDLE            2～5 分钟轮询
         ↓                    ↓
      UID 增量同步 + Message-ID 去重
                    ↓
              邮件 / 线程模型
                    ↓
      邮件列表 / 阅读 / 搜索 / 已读 / 归档
                    ↓
      ┌─────────────┴─────────────┐
      ↓                           ↓
手动转 LifeTrace 行动           手动回复
      ↓                           ↓
任务 / 事件 / 笔记 / 等待事项    SMTP 发送
```

本轮**不做**：

```text
AI 邮件摘要
AI 行动项提取
AI 截止日期提取/推断
AI 自动判断是否需要回复
AI 邮件转任务建议
AI 邮件转事件建议
AI 邮件转等待事项建议
AI 回复草稿
AI Tool Registry 邮件工具
Prompt Injection 与模型上下文相关能力
任何 AI 自动邮件发送
```

这些能力保留在 EPIC-27 后续 AI Extension 中，不作为当前阶段验收条件。

---

## 2. 第一阶段支持范围

必须支持：

```text
QQ 邮箱
163 邮箱
126 邮箱
yeah.net 邮箱
通用 IMAP / SMTP
```

后续再支持：

```text
Gmail API
Gmail Push
Microsoft Graph
Outlook Webhook
```

首期所有国内邮箱都通过统一 IMAP/SMTP Adapter 实现，QQ、163、126、yeah.net 只提供 Provider Preset，不为每个邮箱复制一套业务逻辑。

---

## 3. 当前阶段明确不做

### 3.1 AI 相关

本轮全部暂缓：

- AI 摘要。
- AI 邮件分类。
- AI 行动项提取。
- AI 截止日期提取。
- AI 自动创建任务/事件/等待事项。
- AI 回复草稿。
- AI 邮件工具调用。
- AI 自动发送。
- AI 邮件上下文缓存。
- AI Prompt Injection 防护链路。

注意：虽然本轮不接模型，邮件正文仍然属于**不可信外部输入**，HTML/XSS/远程资源安全仍然必须完成。

### 3.2 其他暂不实现

- Gmail API。
- Microsoft Graph。
- Exchange ActiveSync。
- 完整邮箱规则引擎。
- 批量营销邮件管理。
- 自动清空垃圾箱。
- 自动批量删除邮件。
- 用 LifeTrace 替代专业邮箱客户端的全部功能。

---

## 4. 前置依赖调整

当前非 AI 阶段主要依赖：

### 必须依赖

- **EPIC-12**：文件、附件与对象存储。
- **EPIC-20**：任务、日历、等待事项。
- 现有 Notes 领域能力：邮件手动转笔记。

### 可复用但不要求 AI 能力完成

- **EPIC-21**：如果统一 Domain Service 已存在，应复用，不允许 EPIC-27 直接写其他领域业务表。

### 当前阶段不作为阻塞依赖

- **EPIC-22 AI Tool Registry、权限与执行器中的 AI 部分**。

当前用户手动操作仍应走正常后端权限和 Domain Service，但不需要先完成邮件 AI Tool。

---

## 5. 与现有 LifeTrace 架构的关系

当前仓库已有：

```text
apps/desktop/
services/cloud/
services/cloud/migrations/
services/cloud/src/routes/
services/cloud/src/repository.rs
services/cloud/src/postgres_repository/
services/cloud/src/bin/
services/cloud/src/sync/
services/cloud/tests/
apps/desktop/tests/
```

EPIC-27 必须沿用现有架构，不创建第二套独立 Web 后端。

建议领域边界：

```text
services/cloud/src/mail/
├── mod.rs
├── domain.rs
├── service.rs
├── provider.rs
├── credential.rs
├── imap/
│   ├── mod.rs
│   ├── client.rs
│   ├── sync.rs
│   ├── idle.rs
│   └── parser.rs
├── smtp/
│   ├── mod.rs
│   ├── client.rs
│   └── mime.rs
├── threading.rs
├── sanitizer.rs
└── error.rs

services/cloud/src/routes/mail.rs
services/cloud/src/postgres_repository/mail.rs
services/cloud/src/bin/mail_worker.rs
services/cloud/migrations/<timestamp>_epic27_mail.sql
```

实际编码前先检查现有模块组织和 worker 方式；如果项目已有统一 pattern，以现有 pattern 为准。

---

## 6. 核心数据模型

### 6.1 `mail_accounts`

建议字段：

```text
id
user_id
provider                  # qq / 163 / 126 / yeah / generic
email_address
display_name
imap_host
imap_port
imap_security             # tls / starttls
smtp_host
smtp_port
smtp_security
username
credential_ref            # 仅保存凭据引用
status                    # validating / active / degraded / disabled
last_validated_at
last_sync_at
last_error_code
created_at
updated_at
deleted_at
```

本轮**不添加 `ai_read_enabled`**，避免为暂未实现的 AI 提前污染当前模型。

### 6.2 `mail_folders`

```text
id
account_id
remote_name
normalized_role           # inbox / sent / drafts / trash / spam / archive / other
uidvalidity
uidnext
highest_modseq
last_seen_uid
last_sync_at
sync_enabled
created_at
updated_at
```

约束：

```text
UNIQUE(account_id, remote_name)
```

### 6.3 `mail_threads`

```text
id
user_id
account_id
normalized_subject
latest_message_at
message_count
unread_count
participant_summary
snippet
created_at
updated_at
```

### 6.4 `mail_messages`

```text
id
user_id
account_id
folder_id
thread_id
remote_uid
uidvalidity
message_id
in_reply_to
references_json
subject
normalized_subject
from_json
to_json
cc_json
bcc_json
reply_to_json
sent_at
received_at
flags_json
is_read
is_archived
size_bytes
snippet
body_text
body_html_sanitized
has_attachments
raw_storage_ref
content_hash
created_at
updated_at
```

关键唯一约束：

```text
UNIQUE(account_id, folder_id, uidvalidity, remote_uid)
```

索引至少包括：

```text
(account_id, received_at DESC)
(thread_id, received_at ASC)
(account_id, message_id)
(user_id, is_read, received_at DESC)
```

### 6.5 `mail_attachments`

```text
id
message_id
part_id
filename
mime_type
size_bytes
content_id
disposition
checksum
storage_ref
download_state            # metadata_only / downloading / ready / failed
created_at
updated_at
```

### 6.6 `mail_sync_jobs`

```text
id
account_id
folder_id
kind                       # initial / incremental / reconcile / idle_recovery
state                      # queued / running / success / partial / retry_wait / dead
cursor_before_json
cursor_after_json
attempt
started_at
finished_at
next_retry_at
error_code
error_detail_redacted
created_at
```

### 6.7 `mail_entity_links`

当前阶段不做 `mail_action_proposals`，改为保存用户**手动创建**的实体关联：

```text
id
user_id
thread_id
message_id
entity_type                # task / event / note / waiting
entity_id
created_at
```

用途：

- 从邮件跳转到任务/事件/笔记/等待事项。
- 从目标实体反查来源邮件。
- 防止用户误以为同一邮件还没有转过行动对象。

### 6.8 `mail_drafts` / `mail_outbox`

本轮仍实现普通手动邮件回复，不依赖 AI。

```text
mail_drafts
- id
- user_id
- account_id
- thread_id
- in_reply_to_message_id
- to_json / cc_json / bcc_json
- subject
- body_text
- body_html
- state                    # draft / queued / sent / canceled
- created_by               # 当前阶段固定 user
- created_at / updated_at

mail_outbox
- id
- draft_id
- idempotency_key
- state                    # queued / sending / sent / failed / canceled
- attempt
- provider_message_id
- sent_message_id
- next_retry_at
- last_error_code
- created_at / updated_at
```

---

## 7. 邮箱 Provider 设计

统一 Provider Preset：

```rust
struct MailProviderPreset {
    id: ProviderId,
    imap_host: String,
    imap_port: u16,
    imap_security: SecurityMode,
    smtp_host: String,
    smtp_port: u16,
    smtp_security: SecurityMode,
    display_name: String,
}
```

首期：

- QQ preset。
- 163 preset。
- 126 preset。
- yeah.net preset。
- Generic IMAP/SMTP custom settings。

规则：

- Provider 参数通过官方文档或真实连接测试核对。
- 不允许明文 IMAP/SMTP 密码传输。
- IMAP 和 SMTP 分别测试。
- IMAP 成功、SMTP 失败时账号可进入 `degraded`，仍允许读取邮件。

---

## 8. 凭据安全

定义：

```rust
trait MailCredentialStore {
    async fn put(...)->CredentialRef;
    async fn get(&CredentialRef)->Secret;
    async fn rotate(...);
    async fn revoke(...);
}
```

普通业务数据库只能保存：

```text
credential_ref
```

禁止保存：

```text
password
auth_code
smtp_password
```

要求：

- 加密存储。
- 主密钥不与 ciphertext 放在同一普通数据表。
- Secret 不输出到 `Debug/Display`。
- 日志自动 redaction。
- 删除账号时 revoke。
- 测试和 CI 不提交真实邮箱授权码。

---

## 9. 初次同步

账号连接流程：

```text
用户填写邮箱
  ↓
加载 Provider Preset / Generic 配置
  ↓
CredentialStore 保存授权码
  ↓
IMAP Connection Test
  ↓
SMTP Connection Test
  ↓
Discover Folders
  ↓
保存账号元数据
  ↓
投递 initial_sync
```

首次同步范围：**最近 30 天邮件**。

实现要求：

- 分 folder。
- 分页/分批拉取。
- Inbox/Sent 优先。
- 不允许一次把全邮箱加载到内存。
- 每批成功后推进 cursor。
- Worker 重启后从 cursor 恢复。

---

## 10. UID / UIDVALIDITY / 增量同步

每个 folder 保存：

```text
uidvalidity
last_seen_uid
uidnext
highest_modseq (if supported)
```

规则：

1. UID 是 folder 增量同步主游标。
2. UID 只有在 UIDVALIDITY 不变时有效。
3. UIDVALIDITY 改变时旧 cursor 立即失效。
4. 创建 reconcile job。
5. 使用 Message-ID 和 content hash 辅助重新关联。
6. 数据库唯一约束防止重复插入。

幂等层级：

```text
1. account + folder + uidvalidity + uid
2. Message-ID
3. content hash fallback
```

同一个 sync job 重放两次，最终数据库状态必须一致。

---

## 11. IMAP IDLE 与轮询

Roadmap 要求 2～5 分钟轮询。

建议默认：

```text
MAIL_POLL_INTERVAL_SECONDS=180
```

允许范围：120～300 秒。

连接后检查 capability：

```text
支持 IDLE
  -> 建立 IDLE
  -> 收到 EXISTS 后退出 IDLE
  -> UID 增量同步
  -> 重新进入 IDLE

不支持 IDLE
  -> 自动 polling
```

即使 IDLE 正常，也保留周期性 UID reconciliation。

Worker 必须实现：

- reconnect。
- exponential backoff。
- jitter。
- 最大重试间隔。
- 账号认证失败时停止高频重连。
- 同一 `(account_id, folder_id)` 单实例同步锁/lease。

---

## 12. MIME 和正文解析

支持：

- `text/plain`。
- `text/html`。
- `multipart/alternative`。
- `multipart/mixed`。
- `multipart/related`。
- 常见 charset。
- 编码 Subject。
- 编码 display name。
- inline image / Content-ID。
- 附件 filename 编码。

存储：

- `body_text` 用于搜索。
- `body_html_sanitized` 用于安全展示。
- parser 单封失败不能阻塞整个 folder。
- 错误日志只记录内部 message ID、MIME stage、error code，不记录完整正文。

---

## 13. HTML 安全

必须使用 allowlist sanitizer。

至少禁止：

```text
script
iframe
object
embed
form
onclick/onload/onerror 等事件
javascript: URL
危险 data: URL
危险 style/url()
```

远程图片默认不加载：

```html
<img src="https://tracker.example/pixel?id=...">
```

如后续提供“加载远程图片”，必须由用户显式触发。

禁止服务端因为邮件正文中的 URL 自动发起网络请求，避免 tracking 与 SSRF。

---

## 14. 附件按需下载

同步阶段只保存附件元数据：

```text
part_id
filename
mime_type
size
content_id
disposition
```

用户点击附件时：

```text
权限校验
  ↓
检查 size/mime/filename
  ↓
IMAP fetch part
  ↓
stream 到 EPIC-12 Object Storage
  ↓
checksum
  ↓
storage_ref
```

禁止：

- 用附件 filename 直接拼接服务器路径。
- 无大小上限下载。
- 附件内容写日志。
- 同步时默认下载所有附件。

---

## 15. 邮件线程聚合

优先使用：

```text
Message-ID
In-Reply-To
References
```

算法：

1. `In-Reply-To` 查父 message。
2. 再从 `References` 从近到远查已有 message。
3. 命中则加入同 thread。
4. 未命中再做 subject fallback。

Subject fallback 处理：

```text
Re:
Fwd:
Fw:
答复:
回复:
转发:
```

Fallback 必须保守，宁可拆线程，不要误合并无关邮件。

首期不同邮箱账号之间不自动合并 thread。

---

## 16. 已读、归档和状态同步

首期至少支持：

- 标记已读。
- 标记未读。
- 归档。

正确链路：

```text
UI
 ↓
Mail Domain Service
 ↓
IMAP mutation
 ↓
数据库更新
 ↓
后续 reconciliation
```

禁止只修改 LifeTrace DB 而不修改邮箱服务器，否则下一次同步会状态反弹。

首期不做批量 AI 操作，也不需要实现 AI 删除安全策略。

---

## 17. 搜索

支持过滤：

```text
account
folder
sender
recipient
subject/body keyword
date range
read/unread
has_attachment
```

优先复用现有 PostgreSQL/统一搜索能力，不额外引入独立搜索引擎。

线程列表使用稳定分页，优先 cursor pagination。

---

## 18. 手动转 LifeTrace 行动对象

当前阶段仍保留“邮件行动中心”，但所有转换由用户主动发起，不经过 AI 判断。

### 18.1 邮件转任务

线程详情提供：

```text
[转为任务]
```

点击后弹出表单：

- 标题：默认可使用邮件 Subject，但用户可修改。
- 描述：可选择附带邮件 snippet/链接。
- 截止时间：用户手动填写。
- 优先级：用户手动选择。
- 来源邮件：自动关联。

最终：

```text
Mail UI
  ↓
Task Form
  ↓ user confirm
EPIC-20 Domain Service
  ↓
Task
  ↓
mail_entity_links
```

禁止直接 INSERT EPIC-20 表。

### 18.2 邮件转日历事件

用户手动填写：

- 标题。
- 开始时间。
- 结束时间。
- 时区。
- 地点。

自动保存来源邮件关联。

### 18.3 邮件转笔记

用户选择：

- 使用邮件标题。
- 复制当前选中文本或 snippet。
- 保留邮件链接。

不默认把完整邮件正文复制到 Notes。

### 18.4 邮件转等待事项

用户手动创建：

```text
Waiting Item
- title
- counterparty
- expected_reply_at
- follow_up_at
- source_thread_id
```

后续如果同 thread 出现新邮件，系统可以显示：

```text
“该等待事项关联线程出现新回复”
```

这一判断基于 thread 新消息事件即可完成，不需要 AI。

首期由用户决定是否将 Waiting Item 标记为完成。

---

## 19. 手动回复与 SMTP 发送

当前阶段实现普通回复能力，不生成 AI 草稿。

### 19.1 Reply / Reply All

用户点击：

```text
Reply
Reply All
```

系统根据原邮件头生成初始收件人。

发送前必须完整显示：

- To。
- Cc。
- Bcc。
- Subject。
- 正文。
- 附件。

### 19.2 MIME 构造

正确设置：

```text
From
To
Cc
Bcc envelope
Subject
Date
Message-ID
In-Reply-To
References
Content-Type
charset
attachments
```

回复必须携带 `In-Reply-To` / `References`，保证外部客户端线程聚合正常。

### 19.3 Outbox 幂等

发送链路：

```text
用户点击发送
  ↓
create mail_draft/outbox
  ↓
SMTP worker
  ↓
sent / failed
```

每次发送有唯一：

```text
idempotency_key
```

发送成功记录 Message-ID。

后续 Sent folder 同步时，根据 Message-ID 关联到原 draft/outbox，而不是创建重复业务记录。

网络中断场景必须考虑“服务器已接收但客户端没收到响应”的不确定窗口，重试前优先 reconcile。

---

## 20. Mail Worker

Mail Worker 作为云端长期运行能力。

职责：

```text
账号调度
IMAP connect
initial sync
incremental sync
UID reconciliation
IDLE 生命周期
poll fallback
attachment download jobs
SMTP outbox
retry/backoff
health metrics
```

不负责：

```text
AI 分析
AI Tool 调用
AI 回复生成
自动创建任务
```

建议 job：

```text
mail.initial_sync
mail.incremental_sync
mail.reconcile_folder
mail.refresh_idle
mail.download_attachment
mail.send_outbox
mail.revalidate_account
```

错误分类：

| 类型 | 示例 | 处理 |
|---|---|---|
| transient | timeout、网络断开 | backoff + retry |
| auth | 授权码失效 | account=degraded，通知用户 |
| config | host/port/TLS 错误 | degraded，要求修改配置 |
| data | 单封 MIME 解析失败 | 隔离单封，不阻塞 folder |
| permanent | 不可恢复协议错误 | dead job + error code |

---

## 21. API 能力

具体 URL 命名服从现有 routes 风格。

### Account

```text
POST   /mail/accounts/validate
POST   /mail/accounts
GET    /mail/accounts
GET    /mail/accounts/:id
PATCH  /mail/accounts/:id
DELETE /mail/accounts/:id
POST   /mail/accounts/:id/sync
POST   /mail/accounts/:id/revalidate
GET    /mail/accounts/:id/sync-status
```

### Thread / Message

```text
GET    /mail/threads
GET    /mail/threads/:id
GET    /mail/messages/:id
POST   /mail/messages/:id/read-state
POST   /mail/messages/:id/archive
GET    /mail/messages/:id/attachments
POST   /mail/attachments/:id/download
```

### Manual Actions

```text
POST   /mail/messages/:id/create-task
POST   /mail/messages/:id/create-event
POST   /mail/messages/:id/create-note
POST   /mail/messages/:id/create-waiting-item
GET    /mail/messages/:id/entity-links
```

如果现有 Domain Service 更适合由前端直接调用目标领域 API，则 Mail API 只负责生成 source reference，不重复包装业务接口。

### Draft / Send

```text
POST   /mail/drafts
PATCH  /mail/drafts/:id
POST   /mail/threads/:id/reply
POST   /mail/drafts/:id/send
GET    /mail/outbox/:id
```

所有 API 从登录上下文解析 `user_id`，不信任前端传入 owner ID。

---

## 22. 桌面端 UI

### 22.1 邮箱账号设置

每个账号显示：

```text
邮箱地址
Provider
连接状态
最后同步时间
IMAP 状态
SMTP 状态
```

操作：

- 添加账号。
- 测试 IMAP。
- 测试 SMTP。
- 手动同步。
- 重新认证。
- 断开账号。

授权码：

- password 输入。
- 不回显已有 secret。
- 保存后从前端 state 清除。
- 错误信息不打印 secret。

### 22.2 邮件行动中心

建议三栏结构：

```text
┌──────────────────────────────────────────────────────────┐
│ Mail Action Center                           [同步状态]   │
├──────────────┬───────────────────────┬───────────────────┤
│ 账号/筛选     │ 邮件线程列表           │ 线程详情           │
│              │                       │                   │
│ Inbox        │ Sender                │ 邮件正文           │
│ Unread       │ Subject               │ 附件               │
│ Sent         │ Snippet               │                   │
│              │ Time                  │ [转任务]           │
│              │ unread badge          │ [转事件]           │
│              │                       │ [转笔记]           │
│              │                       │ [等待回复]         │
│              │                       │ [回复] [回复全部]   │
└──────────────┴───────────────────────┴───────────────────┘
```

当前 UI **不显示 AI Summary / AI Action Proposal 区域**。

### 22.3 同步错误

必须区分：

- IMAP auth failed。
- SMTP auth failed。
- TLS failed。
- DNS/network timeout。
- Provider config invalid。
- Sync delayed。
- Mail parser partial failure。

不要全部包装成“无法连接 LifeTrace 云端”。

---

## 23. Android 推送

本轮仍可完成邮件新消息推送，不依赖 AI。

Push payload 仅包含必要信息：

```text
account display name
sender display name
subject/snippet（按隐私设置）
thread_id/deep-link
```

不携带：

- 授权码。
- 完整正文。
- 附件。

IDLE 不可用时，polling 发现新 UID 同样触发推送。

---

## 24. 日志与可观测性

允许日志字段：

```text
request_id
job_id
account_id
folder_id
thread_id
message_internal_id
provider
operation
attempt
duration_ms
result
error_code
```

禁止：

```text
auth_code
password
credential secret
完整邮件正文
完整附件内容
SMTP AUTH payload
```

Metrics 至少：

```text
mail_accounts_active
mail_accounts_degraded
mail_sync_jobs_total{result}
mail_sync_duration_seconds
mail_sync_lag_seconds
mail_messages_ingested_total
mail_messages_deduplicated_total
mail_imap_reconnect_total
mail_idle_sessions_active
mail_poll_fallback_total
mail_smtp_send_total{result}
mail_outbox_queue_age_seconds
```

当前阶段**不需要 AI metrics**。

---

## 25. 测试策略

### Unit Test

必须覆盖：

- Provider preset。
- Credential redaction。
- UID cursor。
- UIDVALIDITY reset。
- Message-ID normalization。
- References / In-Reply-To threading。
- subject fallback。
- MIME parser。
- charset。
- HTML sanitizer。
- attachment filename sanitizer。
- content hash。
- outbox idempotency。
- state machine。

### Integration Test

使用测试 IMAP/SMTP server 或协议 fake。

覆盖：

1. 新账号连接。
2. Folder discovery。
3. 最近 30 天 initial sync。
4. UID incremental sync。
5. 重复轮询不重复保存。
6. UIDVALIDITY 改变后 reconcile。
7. IDLE event 触发同步。
8. IDLE 不可用自动 polling。
9. 断线 reconnect。
10. 坏 MIME 单封隔离。
11. read/unread mutation。
12. archive mutation。
13. attachment on-demand download。
14. SMTP send。
15. SMTP retry/reconcile。
16. Sent Message-ID reconciliation。
17. 邮件手动转 Task/Event/Note/Waiting。

### Security Test

覆盖：

```text
HTML script injection
onerror injection
javascript: link
tracking pixel
path traversal attachment filename
oversized attachment
credential accidentally formatted in error
cross-user account/thread access
```

当前阶段不要求 AI Prompt Injection 测试，因为模型链路尚未实现。

### E2E

```text
添加邮箱
 ↓
连接成功
 ↓
同步最近 30 天
 ↓
收到新邮件
 ↓
线程列表出现
 ↓
打开邮件
 ↓
手动转任务
 ↓
任务中保留邮件来源
 ↓
手动回复
 ↓
SMTP 发送
 ↓
Sent 同步
 ↓
Message-ID 与原 thread 关联
```

---

# 26. 分阶段执行计划

---

## Phase 0：代码审计与接口冻结

- [ ] 阅读 cloud 架构、routes、repository、sync、worker。
- [ ] 阅读 desktop API client/router/state/UI。
- [ ] 核对 EPIC-12/20/Notes/EPIC-21 可复用接口。
- [ ] 确认 CredentialStore 方案。
- [ ] 确认 Mail Worker 部署方式。
- [ ] 冻结 DB schema v1。
- [ ] 冻结 API contract v1。
- [ ] 冻结错误码和状态机。
- [ ] 添加 `mail_epic27` feature flag。

Gate：

- [ ] 不存在明文凭据临时方案。
- [ ] 不依赖任何 AI 接口即可开始开发。
- [ ] 手动转任务等动作有明确 Domain Service 边界。

---

## Phase 1：账号模型 + CredentialStore + Provider

- [ ] migration。
- [ ] repository。
- [ ] CredentialStore。
- [ ] QQ preset。
- [ ] 163 preset。
- [ ] 126 preset。
- [ ] yeah.net preset。
- [ ] Generic IMAP/SMTP。
- [ ] IMAP Test。
- [ ] SMTP Test。
- [ ] folder discovery。
- [ ] account CRUD/revalidate/disconnect。
- [ ] credential revoke。
- [ ] redaction tests。

Gate：

- [ ] 至少一种真实 QQ/网易测试账号连接成功。
- [ ] Generic fake server 集成测试通过。
- [ ] DB/日志均看不到明文凭据。

---

## Phase 2：初次同步 + 增量同步

- [ ] MIME parser。
- [ ] HTML sanitizer。
- [ ] folder cursor。
- [ ] 最近 30 天 initial sync。
- [ ] UID incremental sync。
- [ ] UIDVALIDITY reconcile。
- [ ] Message-ID 去重。
- [ ] content hash fallback。
- [ ] attachment metadata。
- [ ] sync job state machine。
- [ ] lease lock。
- [ ] retry/backoff/jitter。
- [ ] polling scheduler。
- [ ] sync status API。
- [ ] metrics/logging。

Gate：

- [ ] 30 天邮件可恢复地同步。
- [ ] 重复轮询不重复。
- [ ] Worker 重启后继续。
- [ ] 坏邮件不阻塞整个 folder。

---

## Phase 3：线程 + 搜索 + 邮件状态

- [ ] References/In-Reply-To threading。
- [ ] subject fallback。
- [ ] thread aggregate。
- [ ] thread list/detail API。
- [ ] cursor pagination。
- [ ] 搜索过滤。
- [ ] read/unread。
- [ ] archive。
- [ ] reconciliation。

Gate：

- [ ] 标准回复链线程正确。
- [ ] 已读/归档与服务器一致。
- [ ] thread 查询无明显 N+1。

---

## Phase 4：IDLE + Polling + Android Push

- [ ] capability detect。
- [ ] IDLE session。
- [ ] IDLE renew。
- [ ] reconnect/backoff。
- [ ] IDLE -> UID sync。
- [ ] polling fallback。
- [ ] periodic reconciliation。
- [ ] Android push。
- [ ] worker health。

Gate：

- [ ] 支持 IDLE 时接近实时。
- [ ] 不支持 IDLE 自动 polling。
- [ ] 网络中断自动恢复。

---

## Phase 5：桌面端邮件行动中心

- [ ] 邮箱账号设置。
- [ ] 账号状态。
- [ ] 同步状态。
- [ ] thread list。
- [ ] thread detail。
- [ ] sanitized HTML renderer。
- [ ] remote image 默认屏蔽。
- [ ] attachment metadata/download。
- [ ] 搜索/筛选。
- [ ] 已读/归档。
- [ ] loading/error/empty/degraded 状态。

Gate：

- [ ] 用户可以完整连接、同步、阅读、搜索邮件。
- [ ] 恶意 HTML 不执行。
- [ ] UI 中没有未实现的 AI 占位功能干扰主流程。

---

## Phase 6：手动行动转换

- [ ] 邮件转 Task。
- [ ] 邮件转 Event。
- [ ] 邮件转 Note。
- [ ] 邮件转 Waiting Item。
- [ ] `mail_entity_links`。
- [ ] 来源 backlink。
- [ ] thread 新回复提示 Waiting Item。
- [ ] 转换 UI 表单。
- [ ] 重复创建提示。

Gate：

- [ ] 所有转换均由用户主动触发。
- [ ] 通过目标领域 Service 创建。
- [ ] 可从目标对象跳回来源邮件。

---

## Phase 7：手动回复 + SMTP

- [ ] reply/reply-all。
- [ ] draft model/API。
- [ ] recipient 展示和编辑。
- [ ] MIME builder。
- [ ] SMTP client。
- [ ] outbox state machine。
- [ ] idempotency key。
- [ ] send retry/reconcile。
- [ ] Message-ID。
- [ ] Sent folder reconciliation。
- [ ] thread relation。

Gate：

- [ ] 用户可正常手动回复。
- [ ] Reply All 收件人正确展示。
- [ ] 网络异常不会稳定造成双发。
- [ ] Sent 同步后 thread 关联正常。

---

## Phase 8：安全、稳定性和发布

- [ ] cross-user authorization test。
- [ ] credential leak test。
- [ ] log redaction test。
- [ ] HTML/XSS test。
- [ ] SSRF boundary test。
- [ ] attachment security test。
- [ ] large mailbox benchmark。
- [ ] reconnect chaos test。
- [ ] DB index review。
- [ ] worker resource review。
- [ ] backup/restore validation。
- [ ] feature flag rollout。
- [ ] runbook。
- [ ] troubleshooting doc。
- [ ] final E2E。

Gate：

- [ ] 当前非 AI Definition of Done 全部完成。

---

## 27. 当前阶段 Definition of Done

### 账号

- [ ] QQ 或网易邮箱可通过授权码连接。
- [ ] QQ/163/126/yeah.net preset 可用。
- [ ] Generic IMAP/SMTP 可用。
- [ ] IMAP/SMTP 可独立测试。
- [ ] folder discovery 正常。
- [ ] 凭据不出现在普通 DB/日志。
- [ ] 可 revalidate/disconnect。

### 同步

- [ ] 最近 30 天邮件可同步。
- [ ] UID/UIDVALIDITY 正常保存。
- [ ] 增量同步正常。
- [ ] 重复轮询无重复邮件。
- [ ] UIDVALIDITY 变化可恢复。
- [ ] Worker 重启可恢复。
- [ ] 2～5 分钟 polling 正常。
- [ ] IDLE 可用时启用。
- [ ] IDLE 不可用时自动 polling。

### 邮件基础能力

- [ ] thread 聚合正常。
- [ ] MIME 正文解析稳定。
- [ ] HTML 安全清理。
- [ ] remote image 默认不加载。
- [ ] 附件按需下载。
- [ ] 搜索可用。
- [ ] read/unread 可用。
- [ ] archive 可用。

### 行动中心

- [ ] 用户可手动转 Task。
- [ ] 用户可手动转 Event。
- [ ] 用户可手动转 Note。
- [ ] 用户可手动转 Waiting Item。
- [ ] 目标对象保留来源邮件链接。
- [ ] Waiting Item 关联 thread 新回复可提示。

### 回复

- [ ] Reply 可用。
- [ ] Reply All 可用。
- [ ] SMTP 发送可用。
- [ ] outbox 有幂等保护。
- [ ] Message-ID 正确保存。
- [ ] Sent folder reconciliation 正常。

### 安全与质量

- [ ] HTML/XSS 测试通过。
- [ ] SSRF 边界测试通过。
- [ ] cross-user 权限测试通过。
- [ ] credential leak 测试通过。
- [ ] 日志脱敏测试通过。
- [ ] Integration tests 通过。
- [ ] E2E 主闭环通过。
- [ ] 大邮箱 benchmark 已记录。
- [ ] metrics/runbook 完成。

以下项目**不属于当前 DoD**：

```text
AI 摘要
AI 行动提取
AI 截止日期提取
AI Proposal
AI Reply Draft
AI Tool Registry 邮件工具
AI Prompt Injection 测试
```

---

## 28. 后续 AI Extension（暂不执行）

当前阶段全部完成并稳定运行后，再单独启动 AI Extension。

后续可能包括：

```text
邮件 Thread 摘要
重要性判断
行动项提取
截止日期提取
任务/事件/等待事项 Proposal
回复草稿
账号级 AI 读取开关
模型调用缓存
邮件 Prompt Injection 防御
AI Tool Registry
用户确认后的 Tool Execution
```

后续 AI 架构必须遵守：

1. 邮箱凭据绝不进入模型上下文。
2. 邮件正文视为 untrusted content。
3. AI 只生成 Proposal/Draft，不直接产生高风险副作用。
4. 邮件发送不得由模型自动触发。
5. 所有行动通过统一 Domain Service。
6. AI Extension 单独设计 schema、权限、缓存、安全测试和验收，不与当前基础邮件实现混在同一个开发阶段。

---

## 29. Agent 执行规则

1. 先完成 **Phase 0～8 非 AI 能力**。
2. 当前开发过程中不要接 LLM API。
3. 不实现 AI Prompt、模型 schema、AI cache、Tool Registry 邮件工具。
4. 不为了未来 AI 提前增加大量无实际用途的字段和表。
5. 允许保留清晰的扩展接口，但不得影响当前架构简洁性。
6. 账号、同步、线程、SMTP 必须独立于 AI 正常工作。
7. 所有跨领域创建行为仍走 Domain Service。
8. 邮件正文仍按不可信 HTML/外部数据处理。
9. 网络操作必须 timeout + retry budget。
10. 数据库写入必须幂等。
11. 日志不记录邮件正文和凭据。
12. 每个 Phase 完成后运行对应测试。
13. 如果实际项目结构与本文建议冲突，以现有统一架构为准，并同步修正文档。
14. **在用户明确要求启动 AI 部分之前，不进入“后续 AI Extension”。**

---

## 30. 建议提交序列

```text
1. docs(epic27): freeze non-ai mail architecture
2. feat(mail): add mail schema repositories and credential store
3. feat(mail): add provider presets and account validation
4. feat(mail): add imap initial and incremental sync
5. feat(mail): add threading search and mail state mutations
6. feat(mail): add idle worker polling fallback and notifications
7. feat(desktop): add mail action center
8. feat(mail): add manual task event note waiting conversions
9. feat(mail): add manual reply smtp outbox flow
10. test(mail): add security resilience and e2e coverage
11. docs(mail): add operations troubleshooting and release notes
```

AI 相关提交不进入这一批次。

---

## 31. 最终验收场景

### 场景 A：基础同步

1. 添加 QQ 或网易邮箱。
2. IMAP/SMTP 测试通过。
3. 同步最近 30 天。
4. 再次同步无重复。
5. 新邮件通过 IDLE 或 polling 进入系统。
6. thread 正确聚合。

### 场景 B：手动转任务

1. 打开邮件。
2. 点击“转为任务”。
3. 用户填写截止时间和优先级。
4. 创建 Task。
5. Task 中可回到来源邮件。

### 场景 C：等待回复

1. 用户把邮件转 Waiting Item。
2. 对方在同一 thread 回复。
3. LifeTrace 显示“关联线程出现新回复”。
4. 用户决定是否完成 Waiting Item。

### 场景 D：手动回复

1. 用户点击 Reply / Reply All。
2. 编辑收件人和正文。
3. 点击发送。
4. SMTP 发送成功。
5. Sent folder 同步后与原 thread 关联。

### 场景 E：凭据失效

1. 授权码失效。
2. Account -> degraded。
3. Worker 停止高频认证重试。
4. UI 提示重新认证。
5. 日志无授权码。
6. 更新授权码后恢复同步。

### 场景 F：恶意 HTML

邮件包含 script、tracking pixel、javascript URL。

验证：

- UI 不执行脚本。
- 默认不加载 tracking pixel。
- 服务端不自动访问邮件外链。

---

## 32. 当前阶段最终边界

这一阶段的目标可以概括为：

> **先把邮件系统本身做稳定，再谈 AI。**

当前 EPIC-27 首先建立可靠的邮箱协议层、同步层、线程模型、邮件 UI、手动行动转换和 SMTP 回复能力。

只有当这些基础能力稳定、幂等、安全、可观测之后，再启动 AI Extension。这样 AI 后续只是“读取稳定邮件领域数据并提出建议”，不会和 IMAP/SMTP、同步状态机、凭据安全等基础问题耦合在一起。