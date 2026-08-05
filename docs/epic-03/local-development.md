# EPIC-03 本地开发

## 内存原型（当前默认）

```powershell
cargo run --manifest-path services/lifetrace-cloud/Cargo.toml
```

监听 `127.0.0.1:8787`。开发认证默认：

```text
Authorization: Bearer dev-token
```

## Docker 本地环境

```powershell
Copy-Item deploy/cloud/.env.example .env   # 填写后
docker compose -f deploy/cloud/docker-compose.local.yml --profile cloud up -d
```

PostgreSQL 固定 `postgres:16-alpine`、持久化卷、仅回环监听，PostgreSQL 接线后使用。

## 测试

```powershell
cargo test --manifest-path services/lifetrace-cloud/Cargo.toml
```
