# 第四阶段执行记录：附件与实时同步兼容

状态：本批代码完成，生产切流未开始

日期：2026-08-13

## 本批目标

在不修改原版 BeeCount Android/iOS 客户端的前提下，补齐移动端会直接调用的附件和
WebSocket 协议，同时继续坚持 LifeTrace PostgreSQL 单一后端：

- 保持 BeeCount 的 multipart、snake_case 和下载行为；
- 附件不落容器临时磁盘，服务重建后仍可用；
- 新文件必须进入 LifeTrace 的 `file.metadata` 权威变更日志；
- BeeCount 写入和 LifeTrace 原生写入都能唤醒 BeeCount 客户端拉取增量；
- 在兼容面和历史数据校验完成前不修改 Caddy 生产流量。

## 已实现接口

| BeeCount 公开接口 | LifeTrace 内部接口 | 行为 |
|---|---|---|
| `POST /api/v1/attachments/upload` | `.../compat/attachments/upload` | `ledger_id` + `file` multipart、账本内 SHA-256 去重 |
| `POST /api/v1/attachments/batch-exists` | `.../compat/attachments/batch-exists` | 保序返回存在状态、file ID、大小和 MIME |
| `POST /api/v1/attachments/category-icons/upload` | `.../compat/attachments/category-icons/upload` | 用户级分类图标 SHA-256 去重，返回空 `ledger_id` |
| `GET /api/v1/attachments/{file_id}` | `.../compat/attachments/{file_id}` | 所有权校验、原 MIME、UTF-8 文件名下载 |
| `GET /ws?token=...` | `.../compat/ws?token=...` | access token 校验、1008 关闭、ping/pong、同步通知 |

上表中的 `...` 是 `/api/v1/integrations/beecount`。公开路径尚未在 Caddy 重写，当前只通过
内部路径验收。

## 存储和一致性

迁移 `0018_beecount_attachments.sql` 新增 `cloud_file_blobs`：

- 二进制、文件名、MIME、SHA-256、大小和 BeeCount 查找维度都由用户 ID 隔离；
- 交易附件按 `(user_id, ledger_id, sha256)` 去重；分类图标按
  `(user_id, sha256)` 去重；
- 数据库约束验证 SHA-256、字节数、文件名长度和附件类型/账本关系；
- 每次首次上传与 `file.metadata`、`sync_change_log` 在同一事务提交，重复上传只返回已有
  file ID，不制造重复变更。

账务数据依然只存在于 `sync_entities` / `sync_change_log`。`cloud_file_blobs` 是文件字节存储，
不包含账本、交易、账户等第二份业务投影。

## WebSocket 行为

- 查询参数 token 会按 BeeCount 应用 access token 校验，并要求 `sync:write`；
- 缺少、过期、被撤销或属于其他 LifeTrace 应用的 token 会在升级后以 1008 关闭；
- 文本心跳包含 `"ping"` 时返回 `{"type":"pong"}`，同时支持协议级 Ping/Pong；
- BeeCount push 提交后按触及的账本发送 `sync_change`；user-global 实体使用
  `__user_global__`；
- LifeTrace 原生 finance push 提交后查询当前账本列表并发送同形通知，因此 Web 写入可唤醒
  已连接的 BeeCount 客户端；
- 当前 Compose 是单 LifeTrace Cloud 实例，通知总线为进程内广播。未来扩为多副本前必须改成
  PostgreSQL `LISTEN/NOTIFY` 或外部消息总线。

## 安全边界

- 上传默认上限 64 MiB，可用 `BEECOUNT_ATTACHMENT_MAX_UPLOAD_BYTES` 配置，服务启动时限制在
  1 KiB 至 128 MiB；
- 客户端路径会被剥离，文件名最多 255 个字符；响应同时提供安全 ASCII fallback 和 RFC 5987
  UTF-8 文件名；
- 所有附件接口仅接受 BeeCount 应用 token，并分别要求 `files:write` 或 `files:read`；
- 下载按用户过滤 file ID，不通过错误消息泄露其他用户文件；
- 附件 body limit 只放宽在兼容附件 Router，不改变其他 JSON API 的默认限制。

## 验证

- Rust 1.88 `cargo check --tests --locked` 通过；
- Rust 单元测试覆盖 SHA-256、路径文件名、下载头、实时消息字段和配置上限；
- PostgreSQL 端到端测试已编译，覆盖注册、建账本、上传/去重、batch-exists、下载、分类图标
  和 `file.metadata` 数量；有 `TEST_DATABASE_URL` 时执行，无数据库时安全跳过；
- `Cargo.lock` 已锁定 multipart/WebSocket 新依赖；
- Caddy 未切流，旧 BeeCount SQLite 和附件卷没有被修改。

## 下一批门禁

1. profile、devices 和共享账本成员/邀请接口已在第五阶段完成，见
   `phase-5-execution-report.md`；
2. 下一批实现 SQLite + 附件卷的只读盘点、可恢复导入和 SHA-256 对账；
3. 在真实 PostgreSQL 上运行全部迁移和端到端测试；
4. 用原版 Android 客户端验证大文件、离线恢复、token 过期重连和两设备实时同步；
5. 以上通过后才生成 Caddy 路径重写、短暂停写和回滚演练变更。
