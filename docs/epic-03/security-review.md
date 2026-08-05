# EPIC-03 安全检查

## 已落实

- SQL 参数绑定（PostgreSQL 版本接线后生效；迁移 SQL 全部参数化查询设计）
- 认证：Bearer Token，常量时间比较（`subtle::ConstantTimeEq`）
- Cursor / Page Token：HMAC-SHA256 签名，绑定 user + scope
- 用户隔离：所有读取带 `user_id`；相同 Entity/Change ID 跨用户互不影响
- Payload 校验：Entity Registry + Schema Version + 类型化 DTO + payload ID 一致性
- `secret_local_only` 拒绝（未知实体类型即拒绝）
- 生产禁用 DEV_AUTH；生产缺失签名密钥拒绝启动
- CORS：默认关闭，白名单来自配置，不默认 `*`
- 日志：不记录 Bearer Token / DATABASE_URL / 实体正文
- 错误响应不泄露内部 SQL 与目标实体是否存在

## 待 PostgreSQL 接线后复核

- 连接池参数、迁移失败不 Ready、RLS 可选防御层
- 容器非 root 已在 Dockerfile 落实
