# EPIC-03 执行进度报告（已对齐实施方案）

> 日期：2026-08-05  
> 依据：`docs/LifeTrace_EPIC03_Agent_Implementation_Plan.md`（用户上传的完整实施方案）

## 已按方案对齐

- 目录：`services/lifetrace-cloud`（方案 §5）
- 认证边界：AuthProvider + Bearer Token，生产禁用 DEV_AUTH（方案 §11）
- Canonical JSON + Change Hash 幂等（方案 §10）
- 签名 Cursor / Page Token（HMAC-SHA256，绑定 user + scope，方案 §18/20）
- 健康检查 /health/live + /health/ready、/api/v1/meta/version（方案 §12/13）
- 移除 /devices/register（方案 §12 明确不实现）
- PostgreSQL Migration SQL 0001-0006（方案 §9）
- Docker/Compose/Caddy/.env 示例（方案 §24）
- 9 篇 docs/epic-03 文档（方案 §5）

## 本轮完成

1. **独立 Rust/Axum 服务**：`services/lifetrace-cloud`
2. **配置管理**：`CloudConfig::from_env()` + 生产校验（方案 §7）
3. **健康检查**：`GET /health/live`、`GET /health/ready` + `GET /api/v1/meta/version`
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
8. **AuthProvider 边界**：Development / Test 实现，Bearer Token，常量时间比较（EPIC-04 接管）
9. **业务 CRUD 示例（财务）**：`/api/v1/finance/transactions` 创建/列表/查询/删除，
   所有变更走同一同步状态机（版本化、幂等、可被 pull 拉到）
10. **测试**：16 个 API 集成 + 4 个单元测试全部通过（语义与参考 testkit 一致）

## 剩余差异说明

- `pull` / `snapshot` 采用 **POST**（EPIC-02 v1 契约），roadmap 中写的 GET 为旧版描述。
- PostgreSQL + SQLx 运行接线：**未完成**。migration SQL 已就绪；本机 cargo 离线缓存无
  sqlx/postgres 依赖且无法联网拉取，存储层保持可替换设计，接入后公开方法不变。
- 正式认证：**未实现**（属 EPIC-04）；当前为 DevelopmentAuthProvider。

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
