# 本地数据与同步设计

LifeTrace 使用 Dexie 封装 IndexedDB。`activities`、`activityLogs`、`transactions`、`dailyReviews` 分表存储，页面只通过 Zustand action 访问数据，不直接操作数据库。

写入采用本地优先流程：先生成 UUID 并写入 IndexedDB，界面立即更新；启用 Supabase 后，同一条变化进入待同步队列。普通记录按 `updated_at` 最后写入优先；财务数据以新增为主，不覆盖不同 UUID 的交易；删除通过 `deleted_at` 软删除并同步。`sync_events` 的组合唯一约束用于避免重复上传。

当前交付可完整离线使用。填入 Supabase 环境变量并执行迁移后即可接入云端客户端；云端队列调度仍作为下一阶段扩展点。

