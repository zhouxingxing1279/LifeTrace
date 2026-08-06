# EPIC-13 Web / PWA 客户端执行方案

> 状态：实施与 CI 验收中  
> 分支：`feat/epic-13-web-pwa`  
> PR：`#5`  
> 需求来源：`docs/LifeTrace_Complete_Roadmap_v2.md` 的 EPIC-13。

## 1. 已确认的产品决策

EPIC-13 原路线图包含离线壳、本地草稿和 IndexedDB Outbox。产品决策已经调整为：

- Web 端只能联网使用；
- 页面首次进入时直接从 LifeTrace Cloud 加载完整云端快照；
- 财务、笔记、英语等业务数据不写入 IndexedDB、localStorage 或 sessionStorage；
- 新增、编辑、删除和批量导入直接调用 `/api/v1/sync/push`；
- 只有服务器返回 accepted/duplicate 后，页面内存状态才更新；
- 网络或服务器失败时保留当前表单内容，并明确显示“数据未保存”；
- PWA 仅提供安装、快捷入口和版本更新提示，不提供离线业务能力；
- Service Worker 不缓存页面、静态资源或 API 响应。

这一决策优先于原路线图中的离线子任务，避免同时维护浏览器副本、Outbox、冲突重放和隐私清理逻辑。

## 2. 目标架构

```text
React / TypeScript Web UI
        |
        | HttpOnly Cookie + CSRF
        v
LifeTrace Cloud /api/v1
        |
        +-- /web/session/*
        +-- /auth/devices
        +-- /auth/sessions
        +-- /sync/snapshot
        +-- /sync/pull
        +-- /sync/push
        |
        v
PostgreSQL / 对象存储（EPIC-12）
```

浏览器内只保存当前 React 内存状态。刷新页面后重新进行会话恢复和云端快照加载。

## 3. 需求映射

| 路线图能力 | 实现位置 | 状态 |
|---|---|---|
| 独立 Web 入口 | `web-client/`、`vite.web.config.ts` | 已实现 |
| 响应式应用壳 | `App.tsx`、`styles.css`、`epic13.css` | 已实现 |
| 登录 / 安全 Session | `cloud/api.ts`、Web Cookie 路由 | 已实现 |
| 设备与会话管理 | `/devices`、`AuthApi` | 已实现 |
| 隐私模式 | 金额遮罩与界面模糊 | 已实现 |
| 全局搜索 | `cloud/search.ts`、`/search` | 已实现 |
| 快速记账 / 账单列表 | 财务页面 | 已实现 |
| 账户 / 分类 / 预算 | 正式同步实体；预算使用 `user.preference` | 已实现 |
| CSV/XLSX 导入 | `importer.ts` | 已实现 |
| 对账确认 | candidate / confirmed / ignored + 重复检测 | 已实现 |
| 笔记列表 | `/notes` | 已实现 |
| Tiptap / Markdown | `RichTextEditor.tsx`、Markdown 双模式 | 已实现 |
| 文件夹 / 标签 | `note.folder`、`note.tag`、`note.tag_relation` | 已实现 |
| 冲突处理 | 服务器实体覆盖并显示冲突提示 | 已实现 |
| 文章 / 生词 / 高亮 | 英语页面及正式同步实体 | 已实现 |
| 阅读总结 / 统计 | `english.learning_record` | 已实现 |
| Manifest / 安装 | `public/manifest.webmanifest` | 已实现 |
| PWA 快捷入口 | 记账、笔记、英语、搜索 shortcuts | 已实现 |
| 更新提示 | Service Worker waiting worker 提示 | 已实现 |
| 附件上传 | 依赖 EPIC-12 签名上传接口 | 阻塞依赖 |
| 离线壳 / 草稿 / Outbox | 产品决策取消 | 不实施 |

## 4. 云端数据流程

### 4.1 初始加载

1. `GET /api/v1/web/session` 恢复 HttpOnly Cookie 会话；
2. 创建 `CloudDataStore`；
3. 分页调用 `POST /api/v1/sync/snapshot`；
4. 完整应用所有页面后设置 `snapshotCursor`；
5. 数据仅存放于当前页面内存。

### 4.2 保存

1. UI 根据正式 Schema 创建实体 payload；
2. 使用现有 `serverVersion` 作为 `baseServerVersion`；新实体使用 `"0"`；
3. 立即调用 `/api/v1/sync/push`；
4. accepted/duplicate：写入服务器版本并更新界面；
5. rejected：界面不更新，表单不清空；
6. conflict：应用服务器当前实体，记录冲突并要求用户检查后重试。

### 4.3 刷新

页面内刷新使用 `/sync/pull` 按 cursor 顺序应用增量。浏览器刷新或重新登录使用新的完整 snapshot。

## 5. 安全要求

- 密码、access token、refresh token和业务实体不得写入浏览器存储；
- Cookie 始终使用 `credentials: include`；
- Browser sync 写请求携带 `x-csrf-token`；
- Service Worker 不拦截 API 请求；
- 公共设备退出后清空全部 React 内存状态；
- 英语文章正文按文本渲染，不注入不可信 HTML；
- 金额以整数分处理；
- 跨用户实体由云端再次校验所有权和 scope；
- 客户端不得伪造附件上传成功。

## 6. 模块边界

### 云端核心

```text
web-client/src/cloud/
├── types.ts       # 协议、实体类型和公共工具
├── factories.ts   # Schema 对齐的实体工厂
├── api.ts         # Cookie Auth、snapshot/pull/push
└── search.ts      # 全局搜索和导入重复检测
```

### 页面

```text
web-client/src/pages/
├── FinancePages.tsx
├── NotesPage.tsx
├── EnglishPages.tsx
└── DevicesPage.tsx
```

## 7. 自动化门禁

PR 必须通过：

```bash
npm ci
npm run lint
npm run test:unit
npm run web:build
npm run pwa:build
```

同时执行 PostgreSQL browser-cookie sync 集成测试和 Windows/Tauri 回归构建。

## 8. 附件依赖说明

当前 Cloud 路由只有 auth、finance、health、meta、sync 和 web auth，尚无 EPIC-12 所需的签名上传 URL、对象存储写入和下载端点。当前页面展示明确的不可用说明，不创建本地附件副本，也不只同步 `file.metadata` 后声称上传成功。

EPIC-12 完成后接入顺序：

1. 计算 SHA-256；
2. 请求签名上传 URL；
3. 直接上传对象存储；
4. 云端确认后同步 `file.metadata`；
5. 创建笔记与文件关联；
6. 失败时不修改笔记附件状态。

## 9. Definition of Done

- [x] 独立 Web 入口与响应式壳；
- [x] Cookie 会话和 CSRF browser sync；
- [x] 云端 snapshot / pull / push 数据层；
- [x] 设备、隐私和全局搜索；
- [x] 财务核心页面、导入和对账；
- [x] 笔记 Tiptap、Markdown、文件夹和标签；
- [x] 英语阅读、生词、高亮、总结和统计；
- [x] PWA manifest、快捷入口和更新提示；
- [x] 不使用 IndexedDB/localStorage 保存业务数据；
- [x] 自动测试与独立 CI；
- [ ] PR 全部检查通过；
- [ ] 合入 `main`；
- [ ] `main` 合入后检查通过；
- [ ] EPIC-12 完成后接入真实附件上传。
