# EPIC-12 文件服务架构

## 1. 架构边界

EPIC-12 将“业务元数据”和“大文件字节”分开：

```text
业务模块
  │
  ├─ file.metadata / 文件引用 ──> LifeTrace Sync / PostgreSQL
  │
  └─ 文件字节
       │
       ├─ POST /api/v1/files 创建元数据并取得签名
       └─ S3-compatible object storage 直接 PUT / GET
```

Cloud 是权限和元数据权威；对象存储只承担字节存储。客户端永远不持有对象存储永久凭据。

## 2. 数据模型

`file_objects` 是对象生命周期的服务端权威表：

- `user_id`：所有权；
- `domain`：业务领域；
- `original_name` / `mime_type` / `size_bytes`；
- `sha256`：内容身份和完整性；
- `storage_key`：服务端生成的对象 Key；
- `entity_type` / `entity_id`：可选业务关联；
- `status`：`pending | available | failed`；
- `upload_attempts` / `failure_reason`；
- `available_at` / `deleted_at`。

同一用户、同一领域、同一 SHA-256 和大小只保留一个活动对象元数据，从而避免重复上传和重复占用。

## 3. 业务领域

允许的长期云文件领域固定为：

| domain | 用途 |
| --- | --- |
| `finance_imports` | CSV/XLSX 等财务导入文件 |
| `notes_attachments` | 笔记附件 |
| `english_audio` | 英语学习音频/录音 |
| `photos` | 普通同步照片/视频 |
| `workout_imports` | 训记导入证据 |
| `backups` | 备份对象 |

未知 domain 默认拒绝。MIME 也按领域使用白名单，而不是按扩展名放行。

## 4. 上传流程

```text
Client                      Cloud                       S3-compatible storage
  |                           |                                  |
  |-- POST /api/v1/files ---->|                                  |
  |   metadata + sha256       |                                  |
  |                           |-- create/reuse metadata           |
  |<-- file + signed PUT -----|                                  |
  |                                                              |
  |-- PUT signed URL ------------------------------------------->|
  |   x-amz-checksum-sha256                                    |
  |<------------------------------------------------------ 2xx --|
  |                           |                                  |
  |-- POST /complete -------->|                                  |
  |<-- available metadata ----|                                  |
```

元数据先存在，因此网络/对象存储失败不会要求回滚业务实体。失败时客户端调用 `/fail`；再次上传调用 `/upload-url`，Cloud 增加 `uploadAttempts` 并生成新的短时 URL。

## 5. 下载流程

客户端可以通过文件列表、文件详情或既有 `file.metadata` 同步先看到名称、类型、大小、哈希和状态；只有实际需要原文件时才调用：

`POST /api/v1/files/{id}/download-url`

Cloud 只有在文件属于当前用户且状态为 `available` 时返回短时 GET URL。

## 6. S3 兼容配置

Cloud 使用以下环境变量：

```text
FILE_OBJECT_STORAGE_ENDPOINT=https://s3.example.com
FILE_OBJECT_STORAGE_BUCKET=lifetrace-files
FILE_OBJECT_STORAGE_REGION=us-east-1
FILE_OBJECT_STORAGE_ACCESS_KEY_ID=...
FILE_OBJECT_STORAGE_SECRET_ACCESS_KEY=...
FILE_OBJECT_STORAGE_PRESIGN_TTL_SECONDS=900
FILE_MAX_UPLOAD_BYTES=268435456
```

`FILE_OBJECT_STORAGE_ENDPOINT` 必须是无路径、无 query/fragment 的 HTTP(S) origin。生产环境应使用 HTTPS。

对象路径采用 path-style：

```text
/{bucket}/{domain}/{user_id}/{sha_prefix}/{sha256}
```

签名为 AWS Signature Version 4。PUT 签名把 `x-amz-checksum-sha256` 纳入签名头，客户端上传时必须提交该 checksum。

## 7. 权限模型

- 读取：`files:read`；
- 创建、重签、完成、失败、删除：`files:write`；
- 所有 PostgreSQL 查询/更新同时绑定当前 `user_id`；
- 不根据客户端传入的路径读取对象；对象 Key 由服务端构造；
- 未知领域、非法 SHA-256、超限大小、非白名单 MIME 一律拒绝。

因此知道其他用户的文件 UUID 也不能获得其元数据或签名 URL。

## 8. Sync 关系

仓库已有 `file.metadata`：

- ownership: `UserOwned`
- sync mode: `Bidirectional`
- scope: `files:read/files:write`

所以 EPIC-12 不增加新的同步协议。业务模块同步文件元数据/引用，实际字节永远不进入 Push/Pull/Snapshot JSON。

`file_objects` 是对象存储生命周期权威；`file.metadata` 是跨端业务可见的同步描述。客户端或领域服务在完成上传后建立/更新对应文件引用。

## 9. 与现有文件能力的关系

### Photo Staging

`photo_staging` 是临时原图中转：手机/PWA → Cloud 暂存 → Desktop 相册 → ACK 删除。它不等于长期对象存储，也不迁入 `file_objects`。

### BeeCount 附件

BeeCount compatibility 需要保持原协议行为，继续使用其独立附件表/API。后续可以在不破坏客户端协议的前提下把底层 bytes 迁至 EPIC-12，但不是本次切换条件。

### 本地加密私密相册

严格隔离：

- 不创建 `file_objects`；
- 不生成签名 URL；
- 不进入 `file.metadata`；
- 不进入 Sync；
- 不调用远程缩略图、AI 或对象存储。

## 10. 未完成项与后续演进

本轮建立通用云文件核心，但以下客户端消费层能力不能仅靠 Cloud API 宣称完成：

- 通用缩略图生成；
- Windows/Android/Web 的统一本地文件缓存；
- 客户端缓存清理策略；
- S3 对象的后台物理 GC/删除 Provider。

在这些能力正式实现并通过跨端回归前，路线图保持未完成状态。当前 `/orphans` 提供孤立元数据候选检测，为后续 GC 提供输入。
