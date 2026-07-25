# Life trace 个人管理平台

独立的原生 iOS 健身应用源码位于 `ios/HengXuFitness`，包含动作库、训练模板、逐组训练和训练历史。

Life trace 是一个本机个人管理平台，将坚持项目、健身训练、财务与账单、日历和每日复盘整合到同一套桌面与手机界面中。

## 已实现

- 个人总览、快速记录与今日坚持
- 坚持项目创建、编辑、归档和进度记录
- 健身模板、逐组训练、训练历史和自动打卡
- 财务概览、账单搜索编辑、账户管理
- CSV 账单导入、JSON 完整备份与恢复
- 月历、每日复盘
- 三栏笔记工作台、首页快速记录和最近笔记
- Tiptap 富文本、800 ms 防抖自动保存、手动快照与最近 20 个历史版本
- 笔记文件夹、多标签、收藏、置顶、归档、回收站和全文搜索
- 笔记与坚持项目、训练记录、账单之间的多对多关联
- Electron 本机附件（20 MB 类型白名单）及 Markdown、HTML、JSON 导出
- 包含笔记、关联、附件元数据和版本历史的完整 JSON 备份
- D1 本地模式（SQLite）数据层、PWA manifest 与离线壳
- Supabase 客户端入口、完整建表/索引/RLS 迁移

## 运行

要求 Node.js 22.13 或更高版本。

```bash
npm install
npm run dev
npm run pwa:build
npm run pwa:start
npm run https:local
npm run build
npm test
```

电脑管理界面和手机安装入口由同一份稳定生产构建提供。
手机首次安装时连接本地 HTTPS 地址，离线资源准备完成后可以在电脑服务关闭、
手机 Wi‑Fi 关闭的情况下冷启动。再次同步时才需要连接电脑局域网。

复制 `.env.example` 为 `.env.local`，填入 Supabase 项目 URL 与 anon key。不要把 service role key 放进前端。

## Supabase 初始化

1. 创建 Supabase 项目。
2. 在 SQL Editor 执行 `supabase/migrations/001_initial_schema.sql`。
3. 在项目 Authentication 中启用 Email provider。
4. 配置 `.env.local` 后重启开发服务。

迁移创建 profiles、activities、activity_plans、activity_logs、daily_reviews、finance_accounts、transaction_categories、transactions 和 sync_events，并为所有表启用基于 `auth.uid() = user_id` 的 RLS。

## PWA 测试

先执行 `pwa:build`、`pwa:start` 和 `https:local`。手机端只缓存稳定生产构建，
不缓存电脑开发界面。安装入口由支持 PWA 的浏览器提供。

## 数据约定

本地数据库结构见 `db/schema.ts`，运行时会在项目的 `.wrangler` 本地状态目录中创建 SQLite 数据库。网页通过 `/api/state` 访问数据，不把业务数据写入浏览器。金额在前端以十进制输入，云端以整数分 `amount_cents` 保存。

笔记使用独立的规范化表：`notes`、`note_folders`、`note_tags`、
`note_tag_relations`、`note_relations`、`note_attachments` 和
`note_revisions`，迁移见 `drizzle/0007_notes_module.sql`。SQLite 支持时同时
创建 `notes_fts` trigram 全文索引；运行环境不支持该 tokenizer 时会自动使用
参数化 `LIKE` 查询，保证中文搜索可用。列表接口只返回摘要和元数据，打开笔记
时才读取完整正文。

Electron 保持 `contextIsolation: true`、`nodeIntegration: false` 和 sandbox。
renderer 只通过 `desktop/preload.cjs` 暴露的最小 `noteApi` 使用文件选择、附件
打开与导出能力，不能访问任意文件路径或执行命令。附件保存在：

```text
app.getPath("userData")/attachments/notes/{noteId}/
```

笔记正文同时保存 Tiptap JSON、已清理的 HTML、纯文本和 Markdown。JSON 是编辑
数据源，HTML 用于展示/导出，纯文本用于搜索，Markdown 用于可迁移导出。自动
保存会在停止输入 800 ms 后执行；切换笔记、关闭窗口和退出应用前也会提交剩余
草稿。`Ctrl+S` 会立即保存并创建历史快照。

## 笔记快捷键

- `Ctrl+N`：新建正式笔记
- `Ctrl+Shift+N`：新建快速记录
- `Ctrl+S`：立即保存并创建快照
- `Ctrl+Shift+F`：聚焦全局笔记搜索
- `Ctrl+Alt+1`：进入笔记模块
- 编辑器内支持 `Ctrl+B`、`Ctrl+I`、撤销和重做等 Tiptap 标准快捷键

完整平台备份的 `notesBackup` 字段使用 `lifetrace-notes` 版本化格式，包含全部
笔记表。附件二进制文件仍保存在 Electron 用户数据目录；JSON 备份包含附件元
数据，当前版本不会把附件文件压缩进备份。

## 当前边界

当前版本以可靠的单机使用为主。同一台电脑上的不同浏览器访问同一个本地服务时会共享 SQLite 数据。Supabase 邮箱认证、跨设备同步队列和自动更新提示尚未完成。笔记批量 Markdown 导入/导出、附件 ZIP 备份、独立笔记窗口、自定义协议和托盘快速记录属于后续增强；单篇 Markdown 导入的桌面通道已预留。
