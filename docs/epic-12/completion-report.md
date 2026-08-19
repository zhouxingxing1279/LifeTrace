# EPIC-12 文件、附件与对象存储完成报告

## 完成范围

EPIC-12 已按 `execution-plan.md` 完成统一普通文件服务的实现，并保持现有 Photo Challenge 临时中转、BeeCount 附件兼容和私密相册安全边界不变。

### Cloud 文件服务

- PostgreSQL 新增 `cloud_file_objects`，保存用户、领域、文件名、SHA-256、MIME、大小、对象键、状态、上传次数和清理状态。
- 六类受控领域及对象目录：finance imports、notes attachments、english audio、photos、workout imports、backups。
- 同一用户、同一领域按 SHA-256 + 大小去重；不同用户之间不共享去重可见性。
- 复用 EPIC-04 `files:read/files:write` Scope；所有详情、完成、下载和删除查询均同时限制当前用户。
- 对象 key 完全由 Cloud 生成，客户端不能提交任意 bucket path。
- 增加领域级 MIME 白名单和大小限制，并叠加对象存储全局大小上限。
- 上传采用 metadata-first：先创建/复用 metadata，再返回短期 S3 SigV4 PUT URL。
- PUT 签名绑定 `Content-Type`、SHA-256 metadata 和文件领域；complete 阶段通过 HEAD 再校验大小、SHA metadata、领域 metadata 和 MIME 后才进入 `ready`。
- `pending/failed` 可重新初始化并获得新签名 URL；失败不会回滚或删除业务记录。
- 下载只对 `ready` 文件返回短期签名 GET URL。
- DELETE 先提交元数据软删除，再 best-effort 清理对象；对象删除失败进入 `storage_cleanup_pending`，不会把业务删除回滚。
- diagnostics 可发现超时 pending/failed、ready metadata 对应对象缺失和待清理对象。

### Desktop 按需缓存

- 新增通用 content-addressed `file-cache`，文件以 SHA-256 为本地缓存键。
- 原文件下载使用临时文件，流式计算 SHA-256 并校验声明大小；只有完整性一致才原子提交为正式缓存。
- 缓存命中会再次验证大小和 SHA-256，损坏文件不会继续作为有效缓存。
- 相同 SHA-256 复用同一缓存文件。
- 默认 5 GiB 容量上限，按最近访问记录淘汰；清理同时删除可再生缩略图。
- 图片缩略图从已验证原文件本地生成，不写入业务同步或云端对象。

## API

- `GET /api/v1/files`
- `GET /api/v1/files/{id}`
- `POST /api/v1/files/uploads`
- `POST /api/v1/files/{id}/complete`
- `POST /api/v1/files/{id}/download`
- `DELETE /api/v1/files/{id}`
- `GET /api/v1/files/diagnostics`

## 验收标准对应

1. **用户只能访问自己的文件**：文件 metadata API 以认证用户为隔离键；对象 key 包含服务端确定的用户 namespace，客户端不能指定路径。
2. **相同文件可通过哈希去重**：活跃记录使用 `(user_id, domain, sha256, size_bytes)` 唯一索引并在初始化上传时复用。
3. **文件失败不破坏业务记录**：metadata 与字节传输分离，失败保持 `pending/failed` 并支持重试；对象删除失败只记录 cleanup pending。
4. **大文件不进入同步 JSON**：API 和 `file.metadata` 只传稳定 metadata；对象字节通过 S3 兼容预签名 URL 传输。
5. **未下载原文件时仍可查看元数据**：Cloud metadata list/detail 与 Desktop 原文件缓存相互独立。

## 回归边界

- `photo_staging` 继续只用于摄影挑战/照片中转，桌面成功接收后 ACK 删除，未迁移为长期对象库。
- BeeCount stock attachment compatibility 保持原协议，不强制客户端切换新 API。
- EPIC-30 私密相册继续严格 local-only，不获得对象 URL、不进入 Cloud 或文件缓存同步链路。
- EPIC-27 邮件附件 `storage_ref` 仍是独立边界，本 Epic 不伪造其外部对象清理能力。

## 验证门禁

实现过程已经通过定向编译与测试逐层发现并修复以下问题：Cloud 缺失 `reqwest` 直接依赖、Desktop 缺失 `hex` 直接依赖、Rustfmt 表达式不稳定以及 Clippy 未使用字段。修复后 Cloud 的 contracts、文件/对象存储单测、完整 Cloud tests 与 PostgreSQL tests 均已通过；最终合并仍以本报告所在最终提交触发的 Browser Web、EPIC-03 PostgreSQL、EPIC-05 Windows Sync、Local Encrypted Album 四套 GitHub Actions 全绿为硬门禁。

CI run 编号不作为长期文档契约；PR #95 的最终 HEAD 与 GitHub Actions 状态是合并时的权威验证记录。

## 文档

- `docs/epic-12/execution-plan.md`
- `docs/epic-12/architecture.md`
- `docs/epic-12/completion-report.md`
- `docs/roadmap.md` EPIC-12 checklist
- `docs/README.md`
- `services/cloud/README.md`

## 合并策略

PR #95 只有在最终 HEAD 的全部必需 CI 成功、没有临时 workflow 文件残留、分支仍可干净合并到最新 `main` 时才允许合并。