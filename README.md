# LifeTrace

LifeTrace 是一个安静、克制、本地优先的个人长期习惯与生活数据应用，支持桌面与手机浏览器，并可安装为 PWA。

## 已实现

- 今日目标、快速项目记录、行为状态记录
- 项目管理与本地持久化
- 快速记账、月度汇总、最近交易
- 月历、基础趋势统计、每日复盘
- 深浅色模式、网络状态提示、JSON 数据导出
- IndexedDB 本地优先数据层、PWA manifest 与离线壳
- Supabase 客户端入口、完整建表/索引/RLS 迁移

## 运行

要求 Node.js 22.13 或更高版本。

```bash
npm install
npm run dev
npm run build
```

复制 `.env.example` 为 `.env.local`，填入 Supabase 项目 URL 与 anon key。不要把 service role key 放进前端。

## Supabase 初始化

1. 创建 Supabase 项目。
2. 在 SQL Editor 执行 `supabase/migrations/001_initial_schema.sql`。
3. 在项目 Authentication 中启用 Email provider。
4. 配置 `.env.local` 后重启开发服务。

迁移创建 profiles、activities、activity_plans、activity_logs、daily_reviews、finance_accounts、transaction_categories、transactions 和 sync_events，并为所有表启用基于 `auth.uid() = user_id` 的 RLS。

## PWA 测试

先执行生产构建与启动，然后在浏览器开发者工具的 Application 中检查 Manifest、Service Worker 与离线缓存。安装入口由支持 PWA 的浏览器提供。

## 数据约定

本地数据结构见 `src/db/local.ts`，云端映射与冲突原则见 `docs/DATA_AND_SYNC.md`。金额在前端以十进制输入，云端以整数分 `amount_cents` 保存。

## 当前边界

当前版本以可靠的设备内使用为主。Supabase 邮箱认证 UI、完整后台同步队列、CSV 分表导出和自动更新提示已保留数据结构与接入点，但尚未完成产品流程。

