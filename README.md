# LifeTrace

LifeTrace 是一款本地优先的 Windows 个人管理应用，包含习惯、训练、财务、复盘、笔记、每日英语和 iPhone 局域网照片同步。

当前桌面版采用 **Tauri 2 + React/Vite + Rust + SQLite**。正式运行和打包不依赖 Electron、Next.js、Wrangler、Node.js、Python、FFmpeg 或外部数据库。

## 功能

- 习惯项目、打卡、计时与统计
- 训练二维码解析、寻迹导入、训练历史和自动打卡
- 账户、手动记账和微信账单导入
- 日历与每日复盘
- Tiptap 笔记、标签、文件夹、附件、回收站和版本历史
- 英语文章库、VOA 同步、离线词典、生词本和复习计划
- iPhone 快捷指令局域网照片同步、去重、缩略图与媒体预览
- 本地 JSON 备份与恢复

## 新架构

```text
tauri-ui/               Vite 桌面入口和 Tauri API 桥接
src/components/         React 业务界面
src/services/           前端业务服务
src-tauri/src/          Rust 桌面后端
src-tauri/src/server/   SQLite、英语、寻迹、照片同步等本地接口
src-tauri/tauri.conf.json
xunji_service/data/     随安装包发布的离线词典数据库
```

`app/` 只保留 Tauri 前端使用的样式文件。离线词典数据库不会提交到 Git；可按 `xunji_service/data/README.md` 使用公开 ECDICT 数据在本地生成。

## 开发

要求：

- Node.js 22.13 或更高版本
- Rust stable MSVC 工具链
- Windows WebView2（Windows 10/11 通常已内置）

```powershell
npm.cmd install
npm.cmd run dev
```

只启动浏览器界面：

```powershell
npm.cmd run web:dev
```

## 检查

```powershell
npm.cmd test
npm.cmd run test:rust
```

## 打包

```powershell
npm.cmd run build
```

安装包生成在：

```text
src-tauri/target/release/bundle/nsis/LifeTrace_<version>_x64-setup.exe
```

## 数据位置与迁移

新版本数据位于：

```text
%APPDATA%\com.lifetrace.desktop\
```

首次启动时会只读扫描旧版 `%APPDATA%\LifeTrace\wrangler-state`、`%APPDATA%\lifetrace\wrangler-state` 和开发目录 `.wrangler/state`，自动迁移核心数据、英语记录与笔记。旧数据库不会被删除或改写。

卸载应用不会主动删除用户数据库、照片或附件。

### 数据层（EPIC-01）

核心业务已从 `(id, data_json, updated_at)` 迁移到真实列表：

- 财务：`finance_accounts`、`transaction_categories`、`transactions`（金额 `amount_cents` 整数分）、`transaction_evidence`
- 习惯：`activities`、`activity_logs`、`daily_reviews`（同日期唯一约束）
- 笔记：`notes`、`note_folders`、`note_tags`、`note_tag_relations`、`note_relations`、`note_attachments`、`note_revisions`（FTS5 全文检索）
- 英语：`english_articles`、`english_learning_records`、`english_highlights`、`english_notes`、`english_vocabulary`、`vocabulary_occurrences`、`vocabulary_review_state`、`english_ai_analysis`
- 训记：`workout_imports`、`workouts`、`workout_exercises`、`workout_sets`、`training_notes`

所有结构变更通过版本化 Migration 执行（`src-tauri/src/database/`），每次迁移前自动创建
一致性备份（`backups/database/`），失败自动回滚。旧 JSON 表保留为 `legacy_*_json_v1` 供回溯。

详细文档：`docs/epic-01/`（审计、目标 schema、迁移指南、校验报告、回滚指南）。
