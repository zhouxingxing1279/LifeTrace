# EPIC-03 架构说明

## 系统形态

```text
LifeTrace Desktop / Android / Web
                ↓ HTTPS
         lifetrace-cloud (Rust + Axum)
                ↓
           PostgreSQL (sync_entities 等)
```

## 核心决策

1. **服务独立**：云端服务位于 `services/lifetrace-cloud`，不依赖 Tauri/rusqlite/React。
2. **契约复用**：所有 HTTP Wire DTO 直接来自 `lifetrace-contracts`，不定义第二套协议。
3. **通用同步实体存储**：云端权威副本保存在 `sync_entities`（完整经校验的 Entity Payload），
   新实体类型无需重写云端 Repository。
4. **服务端排序**：cursor / server_version 全部由服务端分配，客户端时间仅审计。
5. **显式冲突**：`baseServerVersion` 不匹配即返回 conflict，禁止默认 LWW。

## 模块划分

- `auth/`：AuthProvider 边界（EPIC-04 替换）
- `sync/`：canonical JSON、change hash、签名 cursor、page token
- `store.rs`：内存状态机（实体 / change log / tombstone / 幂等 / 原子组 / snapshot）
- `routes/`：health、meta、sync v1、finance CRUD 示例
- `migrations/`：PostgreSQL Schema（0001-0006）

## 启动流程

配置校验 → 初始化 AuthProvider → 初始化 Codec → 构建 AppState → 构建 Router → 监听 → 优雅关闭。
PostgreSQL Pool/Migration 接线后插入「创建 Pool → 执行 Migration → 就绪检查」。
