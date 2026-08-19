# EPIC-12 文件、附件与对象存储执行计划

## 1. 目标

完成 `docs/roadmap.md` 中 EPIC-12 的统一文件服务，使笔记附件、英语音频、普通照片、财务导入文件、训记导入证据和备份拥有一致的元数据、权限、哈希、对象存储和按需下载边界。

EPIC-12 只处理普通云文件。以下能力保持隔离：

- `photo_staging`：摄影挑战/手机原图进入桌面普通相册前的临时中转，不作为长期对象存储；
- BeeCount compatibility attachments：继续保持 BeeCount 协议兼容边界；
- 本地加密私密相册：严格 `local-only`，不得进入 EPIC-12、Sync、对象存储或远程 URL。

## 2. 现状盘点

仓库已经具备：

- EPIC-03 Rust/Axum + PostgreSQL 云端；
- EPIC-04 用户、Session、设备与 `files:read` / `files:write` Scope；
- EPIC-05 通用 Sync Push/Pull/Snapshot；
- `file.metadata` 已注册为 `UserOwned + Bidirectional` 同步实体；
- `photo_staging` 临时照片中转；
- BeeCount 独立附件实现。

缺口是长期、通用的文件对象生命周期和对象存储传输接口。

## 3. 执行顺序

### Phase A — 元数据与安全边界

1. 新增 PostgreSQL `file_objects` 元数据表。
2. 保存用户、领域、原文件名、MIME、大小、SHA-256、对象 Key、关联业务实体与上传状态。
3. 建立用户/领域/哈希索引与去重约束。
4. 仅允许预定义领域：
   - `finance_imports`
   - `notes_attachments`
   - `english_audio`
   - `photos`
   - `workout_imports`
   - `backups`
5. 文件 API 必须检查 `files:read` / `files:write`，数据库查询必须同时按 `user_id` 过滤。

### Phase B — S3 兼容对象存储

1. 使用 S3 Signature Version 4 生成短时 PUT/GET URL。
2. 上传 URL 绑定 SHA-256 checksum。
3. 对象字节不进入 Axum Sync JSON；客户端直接和对象存储传输。
4. 对象 Key 使用稳定的用户/领域/哈希命名，禁止目录穿越。
5. 对象存储凭据仅保存在 Cloud 环境变量中，不返回给客户端。

### Phase C — 文件生命周期 API

1. `POST /api/v1/files`：先创建/复用元数据，再返回上传签名。
2. `POST /api/v1/files/{id}/upload-url`：上传失败后的短时重签和重试计数。
3. `POST /api/v1/files/{id}/complete`：上传完成后切换为 `available`。
4. `POST /api/v1/files/{id}/fail`：记录失败但不删除业务记录。
5. `POST /api/v1/files/{id}/download-url`：只有 `available` 文件才返回按需下载签名。
6. `GET /api/v1/files` / `GET /api/v1/files/{id}`：只读元数据，不要求下载原文件。
7. `GET /api/v1/files/orphans`：检测长时间未关联业务实体、且上传未完成/失败的候选对象。
8. `DELETE /api/v1/files/{id}`：元数据软删除，外部对象物理回收由后续幂等 GC/生命周期策略负责。

### Phase D — Sync 与客户端边界

1. 继续使用既有 `file.metadata` 双向同步实体，不新增第二套同步协议。
2. 业务实体只保存文件 ID/引用，不内嵌大文件内容。
3. 客户端先持有元数据，再在用户需要内容时请求下载签名。
4. 本地缓存和缩略图属于客户端消费层；若本轮不能在所有客户端安全统一实现，不伪装为已完成，保留为明确后续项。

### Phase E — 验证与文档

1. Rust 单元测试：SigV4、checksum、路径安全、MIME/领域白名单、SHA-256 校验和私密相册隔离。
2. PostgreSQL migration / Cloud tests / Clippy / Rustfmt。
3. 现有 Browser Web、EPIC-03 PostgreSQL、EPIC-05 Windows Sync、Local Encrypted Album 回归。
4. 更新 Cloud README、EPIC-12 架构文档、路线图状态和验证报告。
5. 只有相关 GitHub Actions 全绿后才合并 `main`。

## 4. 验收映射

| EPIC-12 验收项 | 实现方式 |
| --- | --- |
| 用户只能访问自己的文件 | Scope + 所有文件 SQL 强制 `user_id` 条件 |
| 相同文件可通过哈希去重 | `user_id + domain + sha256 + size_bytes` 唯一索引 |
| 文件失败不破坏业务记录 | 文件状态独立为 `pending/available/failed`；失败只记录状态 |
| 大文件不进入同步 JSON | 对象字节使用 S3 签名 URL 直传；Sync 只同步元数据/引用 |
| 未下载原文件仍可查看元数据 | 元数据 API 与 `file.metadata` Sync 独立于对象下载 |

## 5. 不降低安全边界

- 不把 S3 Access Key/Secret 暴露给浏览器或桌面端；
- 不接受任意文件领域；
- 不接受无限大小或任意 MIME；
- 不通过客户端提供的对象 Key 访问存储；
- 不把私密相册接入 Cloud；
- 不把上传成功与业务数据写入绑定成同一脆弱事务。
