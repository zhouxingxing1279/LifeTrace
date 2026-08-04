# LifeTrace EPIC-02 实体注册表

> 权威实现：`crates/lifetrace-contracts/src/registry.rs`
> 每个 entity type 登记：`schemaVersion`、`ownership`、`syncMode`、`conflictMode`、`containsFileReferences`

## 1. 所有权 / 同步 / 冲突模式

| 枚举 | 取值 | 语义 |
|---|---|---|
| `EntityOwnership` | `user_owned` / `server_managed` / `shared_catalog` / `device_local` / `secret_local_only` | 数据归属与保密级别 |
| `SyncMode` | `bidirectional` / `server_to_client` / `client_to_server` / `not_synced` | 同步方向 |
| `ConflictMode` | `optimistic` / `server_authoritative` / `none` | 冲突策略（v1 禁止默认 LWW） |

## 2. 注册表

| entity type | schemaVersion | ownership | syncMode | conflictMode | file refs |
|---|---|---|---|---|---|
| identity.user | 1 | server_managed | server_to_client | server_authoritative | 否 |
| identity.device | 1 | server_managed | server_to_client | server_authoritative | 否 |
| finance.account | 1 | user_owned | bidirectional | optimistic | 否 |
| finance.category | 1 | user_owned | bidirectional | optimistic | 否 |
| finance.transaction | 1 | user_owned | bidirectional | optimistic | 否 |
| finance.transaction_evidence | 1 | user_owned | bidirectional | optimistic | 否 |
| habit.activity | 1 | user_owned | bidirectional | optimistic | 否 |
| habit.log | 1 | user_owned | bidirectional | optimistic | 否 |
| review.daily | 1 | user_owned | bidirectional | optimistic | 否 |
| note.folder | 1 | user_owned | bidirectional | optimistic | 否 |
| note.note | 1 | user_owned | bidirectional | optimistic | 是（附件引用） |
| note.tag | 1 | user_owned | bidirectional | optimistic | 否 |
| note.tag_relation | 1 | user_owned | bidirectional | optimistic | 否 |
| note.relation | 1 | user_owned | bidirectional | optimistic | 否 |
| note.revision | 1 | user_owned | bidirectional | optimistic | 否 |
| english.article | 1 | shared_catalog | server_to_client | server_authoritative | 否 |
| english.learning_record | 1 | user_owned | bidirectional | optimistic | 否 |
| english.highlight | 1 | user_owned | bidirectional | optimistic | 否 |
| english.note | 1 | user_owned | bidirectional | optimistic | 否 |
| english.vocabulary | 1 | user_owned | bidirectional | optimistic | 否 |
| english.vocabulary_occurrence | 1 | user_owned | bidirectional | optimistic | 否 |
| english.vocabulary_review_state | 1 | user_owned | bidirectional | optimistic | 否 |
| workout.import | 1 | user_owned | bidirectional | optimistic | 否 |
| workout.workout | 1 | user_owned | bidirectional | optimistic | 否 |
| workout.exercise | 1 | user_owned | bidirectional | optimistic | 否 |
| workout.set | 1 | user_owned | bidirectional | optimistic | 否 |
| workout.training_note | 1 | user_owned | bidirectional | optimistic | 否 |
| file.metadata | 1 | user_owned | bidirectional | optimistic | 是（文件本体另由 EPIC-12 传输） |
| entity.link | 1 | user_owned | bidirectional | optimistic | 否 |
| user.preference | 1 | user_owned | bidirectional | optimistic | 否 |

共 30 个实体类型；注册表以 `&'static str` 常量与 `REGISTRY` 静态数组提供，`describe()` / `is_syncable()` 查询。

## 3. device_local 与 secret_local_only

以下现有数据**故意不注册为可同步实体类型**，未知 entity type 在服务端一律返回 `LIFETRACE_UNKNOWN_ENTITY_TYPE`，形成纵深防御：

| 类别 | 数据 |
|---|---|
| device_local | 照片与媒体、photo 设备/任务/资产、导入上传文件（`import_uploads`）、AI 会话、英语内容源状态、迁移/备份元数据 |
| secret_local_only | AI API Key（`ai_settings.apiKey`）、翻译 Secret（`translation_settings.secret`）、照片设备 Token（哈希）、本地 TLS 证书私钥、任何 Refresh Token / 邮箱授权码 |

API Key、密码、邮箱授权码、Refresh Token、私钥与证书私钥**永远不得进入普通 Sync Payload**。

## 4. 强制规则（测试覆盖）

- 名称唯一、与 `EntityType::known()` 一一对应。
- `user_owned` 实体必须是 `bidirectional` + `optimistic`（禁止默认 LWW）。
- `server_managed` / `shared_catalog` 使用 `server_authoritative`。
- 五个 ownership 类均可表示（wire 为 `user_owned` 等 snake_case 字符串）。
