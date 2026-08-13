# 第五阶段执行记录：账号设备与共享账本兼容

状态：本批功能代码完成，真实 PostgreSQL 与原版 Android 验收待执行

日期：2026-08-13

## 完成度

按“BeeCount 全量并入 LifeTrace，原版 Android 客户端可直接切换”的最终目标估算：

- 总体约 78%；
- 功能代码约 85%；
- 生产切换就绪度约 65%。

剩余工作集中在旧 SQLite/附件卷导入与对账、真实 PostgreSQL 验收、Caddy 正式改写以及
原版 Android 端到端冒烟，不再是第二套财务后端的功能开发。

## 本批目标

- 对齐 BeeCount profile、头像、devices、共享账本邀请/成员/转让和资源快照接口；
- 复用 `cloud_users`、`cloud_devices`、现有 session/token，不创建第二套账号系统；
- 共享关系只保存权限元数据，账本、交易、账户、分类、标签与预算继续以
  `sync_entities` / `sync_change_log` 为唯一权威副本；
- Editor 写共享账本时写入原账本实体分区，不复制到 Editor 用户分区；
- 成员变更、资料变更、共享资源变更和账本写入继续使用 BeeCount WebSocket 事件字段。

## 已实现接口

下表中的内部前缀为 `/api/v1/integrations/beecount/compat`。

| BeeCount 公开接口 | 内部后端接口 | 结果 |
|---|---|---|
| `GET/PATCH /api/v1/profile/me` | `/profile/me` | 读取/更新昵称、配色、外观、AI 配置和主币种 |
| `POST /api/v1/profile/avatar` | `/profile/avatar` | JPEG/PNG/WebP，最大 1 MiB，版本递增 |
| `GET /api/v1/profile/avatar/{user_id}` | `/profile/avatar/{user_id}` | 公开头像字节与版本缓存头 |
| `GET /api/v1/devices` | `/devices` | BeeCount 设备、系统/机型、IP、时间和 session_count |
| `POST /api/v1/devices/{device_id}/revoke` | `/devices/{device_id}/revoke` | 撤销设备及其 session/access/refresh token |
| `POST/GET /api/v1/ledgers/{id}/invites` | `/ledgers/{id}/invites` | Owner 创建/列出一次性邀请码 |
| `DELETE /api/v1/ledgers/{id}/invites/{code}` | 同形内部路径 | Owner 撤销邀请 |
| `POST /api/v1/invites/{code}/preview` | `/invites/{code}/preview` | 已登录用户预览邀请 |
| `POST /api/v1/invites/{code}/accept` | `/invites/{code}/accept` | Editor 加入，单账本最多 5 人 |
| `GET/PATCH/DELETE /api/v1/ledgers/{id}/members/*` | 同形内部路径 | 列表、角色 no-op、踢出/退出 |
| `POST /api/v1/ledgers/{id}/transfer` | `/ledgers/{id}/transfer` | Owner 与目标 Editor 原子交换角色 |
| `GET /api/v1/ledgers/{id}/shared-resources` | `/ledgers/{id}/shared-resources` | 当前 Owner 的分类、账户和标签快照 |

Caddy 仍未把公开 BeeCount 路径改写到这些内部路径，避免尚未完成存量导入时提前切流。

## 数据模型

迁移 `0019_beecount_profile_collaboration.sql` 新增：

- `beecount_user_profiles`：只保存 BeeCount 专属外观/AI 偏好与头像字节；昵称仍写
  `cloud_users.display_name`；
- `cloud_devices.os_version/device_model`：补足原版设备列表字段；
- `beecount_shared_ledgers`：账本外部 ID 到原实体存储用户的访问登记，不保存账本内容；
- `beecount_ledger_members`：Owner/Editor 角色、邀请来源和加入时间；
- `beecount_ledger_invites`：6 位防混淆邀请码、有效期和一次性使用状态。

Owner 转让只交换成员角色。账本财务实体仍留在原 `storage_user_id` 分区，因此无需搬迁或复制
交易日志；当前 Owner 的 user-global 分类/账户/标签由共享资源快照单独读取。

## 同步与实时行为

- `sync/ledgers` 同时返回本人账本与已加入账本，并给出 `owner` / `editor`；
- `sync/push` 对 ledger-scoped 变更解析成员权限，Editor 写入原账本分区；user-global 变更
  始终属于操作者本人；
- `sync/pull` 可读取本人变更和已加入共享账本的 ledger-scoped 变更；
- `sync/full` 从原账本分区读取交易/预算，从当前 Owner 分区读取账户/分类/标签；
- 共享交易缺少身份字段时，服务端补 `createdByUserId` 和 `updatedByUserId`；
- `sync_change` fan-out 给账本所有成员；资料更新发 `profile_change`；成员操作发
  `member_change`；Owner 的分类/账户/标签兼容写入发 `shared_resource_change`；
- 交易附件按共享账本权限写到原账本文件分区，成员可查重和下载；共享分类图标允许同账本
  成员读取。

## 安全和约束

- 所有写接口只接受 `beecount-mobile` token，并复用 LifeTrace scope；
- 非成员统一按账本不存在处理，避免泄露账本或成员存在性；
- 邀请 1–168 小时有效、单账本最多 10 个活动邀请、最多 5 名成员；
- 接受邀请锁定邀请与账本登记行，避免同一邀请码并发使用；
- 每个账本数据库约束保证恰好最多一个 Owner，转让在单事务内完成；
- 设备撤销同时使相关登录 session 和旋转 token 失效；
- 头像 MIME/扩展名白名单且使用路由局部 body limit，不扩大其他 API 上传面。

## 验证结果

- Rust 1.88 `cargo check --tests --locked` 通过；
- Rust 完整测试运行报告 105 passed、0 failed；
- 新增单测覆盖邀请码归一化和 Profile JSON 校验；
- 新增条件式 PostgreSQL 用例覆盖两用户注册、Profile、头像、设备详情、Owner 建账本、创建
  邀请、Editor 预览/接受、成员列表、共享资源、Editor 写交易、单一实体分区、full 快照和
  Owner 转让；
- 当前执行环境没有 `TEST_DATABASE_URL`，所以 PostgreSQL 用例被测试框架发现并报告通过，
  但其数据库主体提前返回；真实 PostgreSQL 执行仍是上线硬门禁。

## 下一阶段门禁

1. 对旧 BeeCount SQLite 和附件卷执行只读盘点，生成用户/实体/文件数量与 SHA-256 清单；
2. 实现可恢复、分批、幂等的 SQLite → PostgreSQL 影子导入器并逐项对账；
3. 在真实 PostgreSQL 上从空库运行 0001–0019 全迁移和全部条件式集成测试；
4. 用原版 Android 验证登录、Vision 快速记账、离线重放、共享账本、头像/附件、token 过期
   重连和双设备 WebSocket；
5. 全部通过后生成 Caddy 正式路径改写、短暂停写、回滚和旧服务只读保留方案。
