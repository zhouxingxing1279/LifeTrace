# 第三阶段：BeeCount 协议兼容层与 PostgreSQL 统一存储

状态：进行中（认证、核心同步、附件和实时通知已落地，生产切换尚未开始）

## 目标

第三阶段把 BeeCount Cloud 的权威数据从独立 SQLite 迁入 LifeTrace PostgreSQL。原版 BeeCount iOS/Android/Flutter 客户端不需要改代码，仍访问固定的 BeeCount URL；Caddy 仅在最终切换时把这些 URL 重写到 LifeTrace 内部兼容命名空间。

完成后的唯一实体真相源是 `sync_entities` 和 `sync_change_log`。BeeCount 专用表只保存协议兼容元数据，不保存第二份账务投影。

## 不变量

1. BeeCount 公开路径和 snake_case 响应保持兼容。
2. LifeTrace 原有 `/api/v1/auth/*` 和 `/api/v1/sync/*` 契约不改变。
3. 金额进入 LifeTrace 后只用整数分；浮点金额只允许存在于 BeeCount 边界。
4. BeeCount 的 LWW 顺序 `(updated_at, device_id)` 必须可重放；LifeTrace 内部仍使用 `baseServerVersion` 乐观并发控制。
5. 导入的 BeeCount ID 使用 `beecount:` 命名空间，输出给 BeeCount 时再去掉前缀。
6. 切换前 BeeCount SQLite 保持只读可回滚，附件原文件不在验证完成前删除。

## 路由拓扑

| BeeCount 客户端请求 | LifeTrace 内部兼容路径 | 阶段 |
|---|---|---|
| `/api/v1/auth/login` | `/api/v1/integrations/beecount/compat/auth/login` | 已建立 |
| `/api/v1/auth/register` | `/api/v1/integrations/beecount/compat/auth/register` | 已建立 |
| `/api/v1/auth/refresh` | `/api/v1/integrations/beecount/compat/auth/refresh` | 已建立 |
| `/api/v1/auth/logout` | `/api/v1/integrations/beecount/compat/auth/logout` | 已建立 |
| `/api/v1/auth/2fa/status` | `/api/v1/integrations/beecount/compat/auth/2fa/status` | 已建立（返回未启用） |
| `/api/v1/sync/{push,pull,full,ledgers}` | `/api/v1/integrations/beecount/compat/sync/*` | 已建立，待 PostgreSQL 环境验收 |
| `/api/v1/attachments/*` | `/api/v1/integrations/beecount/compat/attachments/*` | 已建立，待 PostgreSQL 环境验收 |
| `/ws` | `/api/v1/integrations/beecount/compat/ws` | 已建立，待反向代理验收 |

Caddy 在切换前不改流量。兼容接口先通过内部路径测试，避免开发期间影响当前 BeeCount Cloud。

## 兼容优先级

### Tier 0：原版移动端必须具备

- 邮箱密码注册、登录、旋转 refresh token、退出登录、设备绑定；
- `sync/ledgers`、`sync/push`、`sync/pull`、`sync/full`；
- 交易附件和分类图标上传、去重检查、下载；
- `/ws?token=...` 的 `sync_change` 通知和 ping/pong；
- profile、devices、共享账本成员/邀请的移动端调用。

### Tier 1：LifeTrace Web 双向账务

- 账本、交易、账户、分类、标签、预算的读写；
- 写入必须经过同一个 LifeTrace sync repository；
- Web 写入能够被 BeeCount pull/WS 观察，BeeCount 写入能够被 LifeTrace sync pull 观察。

### Tier 2：可延后能力

BeeCount 管理后台、AI、汇率代理、备份运维和非核心分析接口不阻塞账务切换；切换期可继续由旧服务提供或明确返回 capability unavailable。

## 数据映射

| BeeCount entity_type | LifeTrace entityType | BeeCount scope |
|---|---|---|
| `ledger` | `finance.ledger` | ledger |
| `account` | `finance.account` | user |
| `category` | `finance.category` | user |
| `transaction` | `finance.transaction` | ledger |
| `tag` | `finance.tag` | user |
| `budget` | `finance.budget` | ledger |
| `exchange_rate_override` | `user.preference` | user |

交易内的 `tagIds` 保留在规范化交易 payload。附件上传会建立 `file.metadata` 权威实体；
BeeCount 交易 payload 中的附件引用继续原样往返。二进制对象保存在同一 PostgreSQL 的
受限文件表，不建立第二份账务投影。

## 并发收敛

BeeCount push 不直接覆盖 `sync_entities`。兼容层先锁定 `beecount_entity_clocks` 对应行：

1. 把客户端时间限制在服务器当前时间之后最多 5 秒；
2. 比较 `(updated_at, device_id)`；旧写入按 BeeCount 语义拒绝但整批继续；
3. 读取当前 LifeTrace `server_version`，构造正常的 `PushRequestV1`；
4. LifeTrace 接受后，在同一事务更新 BeeCount 时钟和 cursor；
5. 遇到并发 version conflict 时重新读取一次并重新执行 LWW 判定，不能无条件重试覆盖。

因此两个协议共享实体真相源，同时各自的冲突规则仍可解释和回放。

## SQLite 迁移与切换

1. **盘点**：记录 SQLite 文件哈希、用户/设备/变更/附件数量和最高 `change_id`。
2. **身份关联**：按已验证邮箱关联 `cloud_users`；同邮箱密码摘要不同必须人工确认，不自动覆盖 LifeTrace 密码。
3. **影子导入**：保留 ID、时间和 tombstone，写入 `sync_entities`；每批更新 `beecount_migration_runs`。
4. **双读比对**：对账本汇总、实体数、金额分合计、附件 SHA-256 做比较；不执行双写。
5. **短暂停写**：冻结旧 BeeCount 写入，导入最终 cursor 增量。
6. **切换**：Caddy 将 BeeCount 主机名的兼容路径重写到 LifeTrace。
7. **观察**：至少验证注册/登录、两设备冲突、离线增量、删除、附件、WS 和 LifeTrace Web 写入回流。
8. **回滚**：恢复 Caddy upstream 到旧服务；旧 SQLite 与附件卷保持原样。切换后的 LifeTrace 新写入需导出为 BeeCount delta 后才允许长期回滚。

## 验收标准

- 原版客户端仅修改服务器地址即可注册或登录；
- BeeCount 与 LifeTrace Web 双向新建、修改、删除交易后最终一致；
- 同一变更重复 push 不产生重复实体；
- pull cursor 分页不丢、不重排，删除保留 tombstone；
- 金额分合计与 SQLite 比对为零差异；
- 附件字节数和 SHA-256 全部一致；
- 回滚演练完成且旧服务数据未被破坏。

## 当前批次

- 已新增 BeeCount 应用身份和最小权限集；
- 已新增内部认证兼容接口，复用 LifeTrace 账户、设备、session 与旋转 token；
- 已新增 `beecount_identity_links`、`beecount_entity_clocks`、`beecount_migration_runs`；
- 已新增 `sync/push`、`sync/pull`、`sync/ledgers`、`sync/full`，所有账务实体写入
  `sync_entities` 与 `sync_change_log`；兼容表只保存 LWW 时钟和来源游标；
- 已实现整数分转换、五秒未来时钟限制、`(updated_at, device_id)` 决胜、幂等重放和
  tombstone；单批 push 在一个 PostgreSQL 事务中提交；
- 已实现可逆 ID 边界：旧 BeeCount ID 在 PostgreSQL 使用 `beecount:`，原生 LifeTrace
  实体在 BeeCount 线协议使用 `lifetrace:`，避免两类 ID 撞名；
- 已加入 PostgreSQL 端到端验收测试，覆盖注册、push、pull、账本、全量快照、重复 push、
  旧写冲突以及 LifeTrace 原生写入回流；
- 已实现交易附件和分类图标 multipart 上传、SHA-256 去重、批量存在性检查、授权下载，
  并为新文件原子写入 `file.metadata` 与 `sync_change_log`；
- 已实现 `/ws?token=...`、1008 策略关闭、JSON ping/pong，以及 BeeCount/LifeTrace 两条
  写入路径到 `sync_change` 的实时通知；
- 第四阶段实现和验证记录见 `phase-4-execution-report.md`；第五阶段 profile/devices、共享
  账本调用和共享同步权限见 `phase-5-execution-report.md`。下一批实现 SQLite 影子导入器；
  Caddy 仍指向旧服务，在 Tier 0 和对账完成前不切流量。
