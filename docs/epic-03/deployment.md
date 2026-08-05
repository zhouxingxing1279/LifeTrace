# EPIC-03 部署说明

## 当前状态

EPIC-04 完成前不得正式公网部署。`deploy/cloud/` 提供本地与生产示例：

- `docker-compose.local.yml`：本地 PostgreSQL + cloud（可选 profile）
- `docker-compose.production.example.yml`：生产示例
- `Caddyfile.example`：本地 HTTPS 反代示例
- `.env.example`：环境变量模板（`.env` 不提交）

## Dockerfile

- 多阶段构建；runtime 无 Rust 工具链；非 root（`lifetrace` 用户）；不复制 `.env`；带健康检查。

## 安全要求

- PostgreSQL 默认仅本机访问；生产 DB 端口不暴露公网。
- 签名密钥只通过环境变量/secret 注入，不进镜像。
