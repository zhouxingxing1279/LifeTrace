# EPIC-12 文件、附件与对象存储执行计划

## 1. 目标

在不改变现有 LifeTrace 同步主协议、不破坏 BeeCount 附件兼容和 Photo Challenge 临时中转链路的前提下，建立统一的长期文件服务。业务实体只同步文件元数据和引用，原始大文件通过 S3 兼容对象存储按需上传、下载和缓存。

本 Epic 覆盖笔记附件、英语音频、普通照片、账单导入文件、训记导入证据和备份文件。私密相册继续保持 local-only，不进入本文件服务。

## 2. 现状与复用边界

- 认证层已经存在 `files:read`、`files:write` Scope，并为 `file.metadata` 预留同步权限。
- Cloud 已有 PostgreSQL、Axum、统一认证/Scope、请求限流和同步基础设施，继续复用。
- `photo_staging` 是摄影挑战原图的临时中转，不作为 EPIC-12 长期对象存储实现；保持现有 API 与桌面拉取行为不变。
- BeeCount attachment API 属于协议兼容面，保持兼容，不强制迁移到新 API。
- 私密相册不接入 Cloud、对象存储、远程 URL 或文件同步。

## 3. 执行阶段

### Phase A：Cloud 文件元数据与对象存储

1. 新增 PostgreSQL 文件元数据表，记录用户、领域、原始文件名、SHA-256、MIME、大小、对象键、状态和上传尝试次数。
2. 定义六类受控文件领域及对象键目录：
   - `finance_import` → `finance/imports`
   - `notes_attachment` → `notes/attachments`
   - `english_audio` → `english/audio`
   - `photo` → `photos`
   - `workout_import` → `workout/imports`
   - `backup` → `backups`
3. 增加 S3 兼容对象存储配置和 AWS Signature V4 预签名能力，不将对象存储凭据下发客户端。
4. 增加 MIME 白名单、领域级大小限制、SHA-256 格式校验和对象键约束。
5. 使用 `(user_id, domain, sha256, size_bytes)` 做用户域内内容去重。

### Phase B：文件 API 与一致性

1. `POST /api/v1/files/uploads`：先创建/复用元数据，再返回签名 PUT URL；重复 ready 文件直接复用。
2. `POST /api/v1/files/{id}/complete`：Cloud HEAD 验证对象大小和 SHA 元数据后标记 ready。
3. `GET /api/v1/files` / `GET /api/v1/files/{id}`：只返回当前用户可见元数据。
4. `POST /api/v1/files/{id}/download`：仅 ready 文件返回短期签名 GET URL。
5. 上传失败通过重复初始化获得新的签名 URL，并增加 attempt；失败不得回滚或破坏业务记录。
6. 删除采用元数据软删除 + 对象 best-effort 清理；业务实体与文件生命周期解耦。
7. 增加孤立/异常对象诊断：过期 pending、ready 但对象缺失等状态可被检测。

### Phase C：客户端按需缓存

1. 业务同步只携带 `file.metadata` / file id 等小型结构，不把文件字节放入同步 JSON。
2. 桌面端增加通用下载缓存：按需获取签名 URL、下载到临时文件、校验 SHA-256 后原子落盘。
3. 本地缓存以内容哈希复用，避免同一文件重复占用磁盘。
4. 增加按最后访问时间和总容量的缓存清理。
5. 对支持的图片生成本地缩略图缓存；缩略图是可再生缓存，不进入业务同步。

### Phase D：文档、回归与合并

1. 更新 `docs/epic-12/architecture.md`、`docs/README.md`、Cloud/部署配置说明及 roadmap 完成状态。
2. 增加 Cloud 单元/集成测试与桌面缓存测试。
3. 运行与本次变更相关的 Rustfmt、Cloud tests、Clippy、Browser Web、PostgreSQL、Windows Desktop/Sync 等 CI 门禁。
4. 仅在最终提交对应的必需 CI 全绿后合并 `main`。

## 4. 安全与数据约束

- 每个文件 API 均以认证用户为第一隔离键；不得通过 UUID 猜测读取其他用户文件。
- 文件领域必须来自固定枚举，客户端不能自定义对象目录。
- 对象键由服务端生成，不接受客户端传入路径，防止目录穿越和跨租户覆盖。
- S3 Access Key/Secret Key 仅存在 Cloud 环境变量；客户端只获得短期、限定 method/path 的签名 URL。
- 上传前校验 MIME、大小、SHA-256；完成后再次通过对象 HEAD 验证已上传对象的声明属性。
- 大文件原文不进入 `sync_entities`、日志、错误响应或诊断包。
- 私密相册与 EPIC-12 保持硬隔离。

## 5. 测试计划

### Cloud

- 文件领域/MIME/大小/SHA 校验。
- 同用户同领域哈希去重；不同用户不互相去重或读取。
- 跨用户 metadata/download/complete/delete 返回不可访问。
- 预签名 URL 的 canonical request、过期时间、对象路径和 method 正确。
- 上传 retry 生成新签名并增加 attempt。
- 完成上传前后状态机正确；对象校验失败不进入 ready。
- pending/缺失对象诊断可识别孤立文件。
- metadata API 返回引用信息而非文件字节。

### Desktop

- 下载后 SHA-256 不一致时拒绝进入缓存。
- 相同哈希命中已有缓存。
- 临时文件失败不会留下伪成功缓存。
- 缓存上限清理可重复执行。
- 图片缩略图可生成、失效后可重建。

### 回归

- 现有 `photo_staging` relay 测试继续通过。
- BeeCount attachment compatibility 测试继续通过。
- Browser/Cloud auth/sync、EPIC-03 PostgreSQL、EPIC-05 Windows Sync、Local Encrypted Album 等相关工作流无回归。

## 6. 完成定义

EPIC-12 只有在以下条件同时满足时才标记完成：

- Roadmap 中 Cloud 文件服务、文件同步和六类存储目录的实现均有代码或明确的可验证实现依据。
- 五条 Epic 验收标准均被自动化测试或架构约束覆盖。
- `docs/epic-12/` 有执行计划、架构说明和完成报告。
- 最终 PR 的相关 CI 全部成功。
- 合并使用经过 CI 验证的确切提交，随后确认 `main` 已包含该提交。
