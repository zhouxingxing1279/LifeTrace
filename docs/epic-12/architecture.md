# EPIC-12 文件、附件与对象存储架构

## 1. 职责边界

EPIC-12 为 LifeTrace 的普通文件提供统一长期文件服务。业务实体只保存文件 ID、元数据和关联关系；原始字节不写入同步 JSON，而是进入 S3 兼容对象存储。

覆盖领域：

| 领域 | API domain | 对象键目录 |
| --- | --- | --- |
| 财务导入 | `finance_import` | `finance/imports` |
| 笔记附件 | `notes_attachment` | `notes/attachments` |
| 英语音频 | `english_audio` | `english/audio` |
| 普通照片 | `photo` | `photos` |
| 训记导入证据 | `workout_import` | `workout/imports` |
| 备份 | `backup` | `backups` |

以下能力不迁入本服务：

- 私密相册继续严格 `local-only`，不得生成远程 URL、上传对象存储或进入同步。
- Photo Challenge 的 `photo_staging` 继续作为临时原图中转，桌面 ACK 后删除；它不是长期文件库。
- BeeCount attachment API 继续保持协议兼容，不强制客户端切换接口。

## 2. 数据模型

PostgreSQL `cloud_file_objects` 保存：

- `id` / `user_id`
- 固定 `domain`
- `original_name`
- `sha256`
- `size_bytes`
- `mime_type`
- 服务端生成的 `object_key`
- `pending / ready / failed / deleted` 状态
- 上传尝试次数、失败信息和对象清理状态
- 创建、更新、ready、删除时间

活跃文件使用 `(user_id, domain, sha256, size_bytes)` 唯一索引。去重只发生在同一用户、同一领域内，不会跨用户暴露文件存在性。

## 3. 上传状态机

```text
业务准备文件
   ↓
POST /api/v1/files/uploads
   ↓
Cloud 校验用户 / Scope / domain / MIME / size / SHA-256
   ↓
先写 cloud_file_objects(pending)
   ↓
返回短期 S3 SigV4 PUT URL
   ↓
客户端直传对象存储
   ↓
POST /api/v1/files/{id}/complete
   ↓
Cloud HEAD 对象并校验大小 + SHA 元数据 + domain 元数据
   ↓
ready
```

同一内容已经 `ready` 时，初始化接口直接返回已有元数据，不再要求重复上传。未完成或失败上传再次初始化会增加 `upload_attempts` 并获得新的短期签名 URL。

业务记录与文件传输分离：上传失败只让文件保持 `pending/failed`，不会回滚或删除已经存在的业务实体。

## 4. 下载与桌面缓存

只有 `ready` 文件可以获得短期签名 GET URL。Desktop 的 `file-cache` 使用内容寻址路径：

```text
<data-dir>/file-cache/
├── objects/<sha-prefix>/<sha>.blob
├── thumbnails/<sha-prefix>/<sha>.jpg
└── access/<sha>.touch
```

下载流程边写临时文件边计算 SHA-256，并同时校验声明大小。只有二者完全匹配后才原子重命名为正式缓存文件；失败的临时文件不会成为缓存命中。

相同 SHA-256 复用同一本地缓存。缓存按最近访问标记和总容量清理，缩略图随原文件淘汰。图片缩略图是可再生派生数据，不进入云同步。

## 5. 用户与权限隔离

文件 API 使用现有 EPIC-04 Scope：

- `files:read`
- `files:write`

所有元数据查询、完成、下载、删除都同时限定 `id + user_id`。对象键完全由服务端生成：

```text
users/{user_id}/{domain-prefix}/{sha-prefix}/{sha256}
```

客户端无法指定 bucket path，因此不能通过路径穿越或自定义 key 覆盖其他用户对象。

Finance、Notes、English、BeeCount、Desktop 和 Web 按业务需要获得文件 Scope；Habits 不因 EPIC-12 自动获得文件权限。

## 6. S3 兼容配置

Cloud 从环境变量读取：

- `OBJECT_STORAGE_ENDPOINT`
- `OBJECT_STORAGE_REGION`
- `OBJECT_STORAGE_BUCKET`
- `OBJECT_STORAGE_ACCESS_KEY`
- `OBJECT_STORAGE_SECRET_KEY`
- `OBJECT_STORAGE_PRESIGN_TTL_SECONDS`（默认 900 秒，限制 60–3600 秒）
- `OBJECT_STORAGE_MAX_FILE_BYTES`（全局上限；各领域还有更小的领域上限）

如果对象存储变量全部为空，普通 Cloud 能继续启动，但 EPIC-12 文件传输接口返回服务不可用；这避免在未配置对象存储时静默把大文件写入 PostgreSQL。

生产 `docker-compose.production.yml` 已使用 `.env.production` 注入 Cloud 环境变量，因此凭据只需要写入服务器的 `.env.production` 或外部 Secret 管理系统，不写入仓库。

### Bucket CORS

Web 浏览器直接使用预签名 URL 时，对象存储 Bucket 必须允许 LifeTrace Web Origin 的短期 `PUT / GET / HEAD`，并允许上传请求携带服务端签名返回的 `x-amz-meta-*` 请求头。不要使用公网匿名读写策略；访问控制依赖私有 Bucket + 短期 SigV4 URL。

Desktop 使用原生 HTTP 客户端下载，不依赖浏览器 CORS。

## 7. API

| 方法 | 路径 | Scope | 说明 |
| --- | --- | --- | --- |
| GET | `/api/v1/files` | `files:read` | 当前用户文件元数据列表 |
| GET | `/api/v1/files/{id}` | `files:read` | 单个文件元数据 |
| POST | `/api/v1/files/uploads` | `files:write` | 创建/复用元数据并获取 PUT URL |
| POST | `/api/v1/files/{id}/complete` | `files:write` | 完成对象校验并进入 ready |
| POST | `/api/v1/files/{id}/download` | `files:read` | 获取短期 GET URL |
| DELETE | `/api/v1/files/{id}` | `files:write` | 软删除元数据并 best-effort 删除对象 |
| GET | `/api/v1/files/diagnostics` | `files:read` | stale pending、缺失 ready 对象、待清理对象诊断 |

DELETE 首先提交元数据删除，再尝试删除对象。对象删除失败只设置 `storage_cleanup_pending`，不会让业务删除事务回滚。

## 8. 与同步协议的关系

`file.metadata` 已在现有同步 Scope 映射到 `files:read/files:write`。同步层只承载文件元数据/引用；原始字节始终通过对象存储 URL 传输。因此：

- 大文件不会进入 `sync_entities` 请求 JSON；
- 未下载原始文件时仍可展示文件名、大小、MIME、状态和业务引用；
- 新设备可以先得到业务数据与文件元数据，再按用户操作下载文件。

## 9. 完整性与已知边界

Cloud 完成阶段验证对象存储 HEAD 返回的大小以及上传时写入的 SHA/domain 元数据；Desktop 真正下载字节时重新计算 SHA-256。这样对象传输错误不会被写成有效本地缓存。

S3 兼容服务对服务端计算 checksum 的支持不一致，因此当前不依赖 ETag 作为 SHA-256。若后续统一到支持 `x-amz-checksum-sha256` 的对象存储，可在保持 API 兼容的前提下增加服务端字节级 checksum 验证。

## 10. 运维诊断

`/api/v1/files/diagnostics` 支持发现：

- 超过 24 小时仍为 `pending/failed` 的文件；
- 元数据为 `ready` 但对象 HEAD 已不存在的文件；
- 元数据已删除但对象清理仍待重试的文件。

这些诊断结果可被 EPIC-15 数据健康中心进一步聚合。