# EPIC-05 用户归属模型

## 四种身份

| 身份 | 来源 | 保存位置 | 用途 |
|---|---|---|---|
| `LocalProfileId` | 客户端生成 UUID | SQLite `local_profiles.id`、业务表 `user_id` | 本地数据隔离 |
| `CloudUserId` | `/api/v1/auth/me` 或 Refresh 响应 | SQLite Profile 绑定字段；会话内存 | 关联本地 Profile 与云账号 |
| `AppId` | 客户端常量 | 请求 DTO | 区分桌面、未来移动端等应用 |
| `DeviceId` | 安装实例生成 | 客户端设置/会话 | 幂等、审计与设备管理 |

## 不变量

1. 本地业务表中的 `user_id` 统一解释为 `LocalProfileId`，不是 Cloud User ID。
2. Cloud 所有权只来自 Access Token 的认证 Principal；Payload 中的 `userId` 不参与授权。
3. `sync_set_session` 不接受 Cloud User ID。客户端先调用 `/auth/me`，使用服务器返回的用户身份。
4. `sync_bind_current_profile` 和 `sync_create_cloud_profile` 不接受 Cloud User ID 参数，避免前端伪造绑定目标。
5. 一个 Cloud User 最多绑定一个本地 Profile；数据库唯一约束拒绝重复绑定。

## 首次登录

登录成功只建立内存会话并检查 Active Profile 是否已绑定当前 Cloud User。若未绑定，Profile 标记为 `pending_choice`，前端必须让用户二选一：

- **绑定当前本地档案**：保留 LocalProfileId，将其绑定到已验证 CloudUserId，并为现有可同步实体创建初始 Outbox；
- **创建新的云端档案**：创建空 Local Profile、绑定当前 CloudUserId、切换 Active Profile，然后通过 Snapshot 初始化。

不会在登录成功时自动上传历史数据。

## 切换账号与退出

切换账号时，服务器身份验证结果决定可见 Cloud 绑定。Repository 始终按 Active LocalProfileId 查询，因此不同 Profile 的本地数据不会混合。退出登录只清除内存 Access Token 和 Credential Manager 中的 Refresh Token/会话状态，不删除本地 Profile 或业务数据；离线功能继续可用。
