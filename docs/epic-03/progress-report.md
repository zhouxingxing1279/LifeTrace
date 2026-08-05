# EPIC-03 执行进度报告

> 日期：2026-08-05  
> 依据：`docs/LifeTrace_Complete_Roadmap_v3.md` 中 EPIC-03 章节 + EPIC-02 文档中
> 「EPIC-03 云端服务需要实现的接口」边界（四个端点 + 服务端状态机）

## 本轮完成

1. **独立 Rust/Axum 服务**：`crates/lifetrace-sync-server`
2. **配置管理**：`Config::from_env()`，支持监听地址与全部限制参数
3. **健康检查**：`GET /healthz`
4. **优雅关闭**：Ctrl+C / SIGTERM 时正常退出
5. **请求 ID**：`X-Request-Id` 生成与透传（tower-http）
6. **同步 API（协议 v1）**：
   - `GET  /api/v1/sync/capabilities`
   - `POST /api/v1/sync/push`
   - `POST /api/v1/sync/pull`
   - `POST /api/v1/sync/snapshot`
7. **服务端状态机**（`src/store.rs`，按用户隔离）：
   - 实体存储（payload 校验、payload ID 与 change entityId 一致性）
   - 服务端 change log + cursor（严格升序、分页）
   - tombstone（删除进入 change log、双方删除幂等）
   - 幂等（changeId + payload 一致返回 duplicate；不同 payload 拒绝）
   - 原子组（同组全成或全败）
   - 冲突（baseServerVersion 不匹配显式返回，不做 LWW）
   - Snapshot（一致视图、分页、完成后从 snapshotCursor 继续 Pull 无缝）
   - 错误码（协议/版本/批量/游标/实体等稳定码）
8. **设备注册占位**：`POST /api/v1/devices/register`（EPIC-04 接管）
9. **业务 CRUD 示例（财务）**：`/api/v1/finance/transactions` 创建/列表/查询/删除，
   所有变更走同一同步状态机（版本化、幂等、可被 pull 拉到）
10. **测试**：15 个 HTTP 集成测试全部通过（语义与参考 testkit 一致）

## 与 roadmap 清单的差异说明

- `pull` / `snapshot` 采用 **POST**（EPIC-02 v1 契约），roadmap 中写的 GET 为旧版描述。
- PostgreSQL + SQLx：**未实现**。本机 cargo 离线缓存无 sqlx/postgres 依赖且无法联网拉取，
  因此存储先做内存实现 + 可替换设计；接入 PostgreSQL 时公开方法不变。
- 正式认证：**未实现**（属 EPIC-04）；当前用 `X-LifeTrace-User` 头占位并保证用户隔离。

## 验收标准核对

- [x] 云端保存完整业务副本 —— 内存存储可保存全部同步实体（持久化待 PostgreSQL）
- [ ] 数据库端口不暴露公网 —— 待部署阶段（当前仅监听回环地址）
- [ ] 所有业务接口必须认证 —— 待 EPIC-04
- [x] 不同用户数据严格隔离 —— 按 UserId 分存储，测试覆盖
- [x] push、pull 和 snapshot 可稳定运行 —— 15 个集成测试通过
- [x] 服务重启后同步状态不丢失 —— 内存存储不满足，待持久化；状态机本身重启恢复路径已设计

## 下一步

1. 接入 PostgreSQL + SQLx migration（恢复联网或本机安装依赖后）
2. EPIC-04 认证/Token 接入后替换用户解析
3. 业务 CRUD 模块逐个实现（finance 已完成示例，其余模块复用同一模式）
4. 部署与 CI（EPIC-18/19）
