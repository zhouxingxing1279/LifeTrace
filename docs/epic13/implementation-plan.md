# EPIC-13 Web / PWA 客户端完整执行方案

> 状态：实施中  
> 目标分支：`feat/epic-13-web-pwa`  
> 基线：`main`  
> 需求来源：`docs/LifeTrace_Complete_Roadmap_v2.md` 的 EPIC-13 以及已完成的 EPIC-03/04/05 公共云、认证和同步协议。

## 1. 目标与边界

EPIC-03 已完成 PostgreSQL 云端数据层、认证基础设施和 `/api/v1/sync/*` 协议；EPIC-04/05 已完成统一账户与多端同步。本 EPIC 不重复建设后端，而是在这些能力上交付可部署、可安装、可离线工作的 Web/PWA 客户端。

本次必须交付：

1. React + TypeScript + Vite 的独立 Web 入口；
2. Cookie 会话登录、恢复、退出和 CSRF 防护；
3. 财务账户与流水管理；
4. 笔记创建、编辑、搜索、置顶和删除；
5. 英语文章阅读与词汇管理；
6. 本地缓存、离线 outbox、联网同步和冲突处理；
7. PWA manifest、Service Worker、响应式移动端体验；
8. 单元测试、双前端构建回归、独立 GitHub Actions；
9. 部署、回滚和验收文档。

明确不在本 EPIC 内实现：用户注册、密码找回、富文本协同编辑、财务导入、英语文章后台生产、附件离线缓存、服务端部署自动化。上述能力继续由对应后续 EPIC 承担。

## 2. 需求到实现的映射

| 路线图任务 | 实现 | 验收证据 |
|---|---|---|
| T13.1 Web 工程基础 | `web-client/`、`vite.web.config.ts`、独立构建输出 `dist-web/` | `npm run pwa:build` |
| T13.2 登录/登出/会话 | `AuthApi` 调用 `/api/v1/web/session/*`；HttpOnly Cookie；CSRF Header | Auth API 单元测试、浏览器联调 |
| T13.3 财务 Web | 账户创建、收入/支出记录、列表、删除、金额分转化 | Schema 工厂测试、交互验收 |
| T13.4 笔记 Web | 创建、编辑、搜索、置顶、删除 | 离线队列测试、交互验收 |
| T13.5 英语学习 Web | 只读文章目录/阅读器、生词添加、掌握状态切换 | Schema 工厂测试、交互验收 |
| T13.6 离线最低可用 | localStorage 实体缓存 + outbox；Service Worker App Shell；online/offline 状态 | 离线恢复测试、PWA 产物检查 |
| T13.7 性能/兼容/部署 | 无新增运行时依赖、按路由单入口轻量构建、5 MB 产物门禁、部署文档 | CI 构建及体积门禁 |

## 3. 技术架构

### 3.1 目录

```text
web-client/
├── index.html
└── src/
    ├── main.tsx          # React 入口与全局样式
    ├── App.tsx           # 页面、模块和响应式导航
    ├── core.ts           # Auth、同步、离线仓库、实体工厂
    └── styles.css        # 桌面/移动端样式
public/
├── manifest.webmanifest
└── sw.js
vite.web.config.ts
```

Web 与 Tauri 前端共用根 `package.json` 和 React 依赖，但使用独立 Vite root 和输出目录，避免改动桌面端入口与发布流程。

### 3.2 认证

- 登录：`POST /api/v1/web/session/login`；
- 恢复：`GET /api/v1/web/session`；
- 退出：`POST /api/v1/web/session/logout`；
- Cookie 始终使用 `credentials: include`；
- 退出等状态修改请求携带 `x-csrf-token`；
- 页面不持久化访问令牌，不读取 HttpOnly Cookie；
- 本地仅缓存非敏感会话显示信息，用于断网后进入已缓存工作区；
- 在线退出后清理业务缓存，确保不能继续访问本地数据。

### 3.3 同步与本地数据

Web 客户端身份固定为：

```json
{
  "appId": "lifetrace-web",
  "platform": "web",
  "clientVersion": "0.2.1",
  "protocolVersion": 1,
  "schemaVersion": 1
}
```

同步顺序：

1. 本地修改立即写入实体缓存；
2. 为每次修改生成唯一 `changeId` 并追加 outbox；
3. 联网后先 push，防止 pull 覆盖本地未提交修改；
4. push 成功后记录 `serverVersion` 并移除对应 outbox；
5. 再按服务器 cursor 顺序 pull；
6. 完整应用一批后才保存 `nextCursor`；
7. 若发生乐观锁冲突，采用服务器当前实体，并在本地记录冲突摘要；
8. 删除使用 tombstone 语义，本地立即隐藏实体。

同步实体范围：

- `finance.account`
- `finance.transaction`
- `note.note`
- `english.article`（只读）
- `english.vocabulary`

所有 payload 使用 EPIC-02 JSON Schema 的 camelCase 字段；金额使用整数分；`serverVersion` 和 cursor 使用字符串。

### 3.4 离线与 PWA

- Service Worker 缓存 App Shell、manifest 和图标；
- 导航请求优先网络，失败后回退缓存首页；
- 静态资源 cache-first，API 请求不进入 Service Worker 缓存；
- localStorage 按 `userId` 隔离业务缓存；
- 稳定 deviceId 单独保存；
- 离线编辑只进入 outbox，不伪造服务器版本；
- 页面显示在线状态和待同步数量；
- 公共设备退出时同样清理本地缓存。

## 4. 分阶段执行

### 阶段 A：契约核对

- 核对 EPIC-03 完成报告和路由；
- 核对 Web Cookie/CSRF 认证端点；
- 核对 AppId、scope、sync v1 DTO；
- 核对五类实体 Schema 和读写方向。

完成条件：代码不使用猜测的端点、实体名或字段。

### 阶段 B：工程和核心能力

- 建立独立 Web Vite 入口；
- 实现 `AuthApi`；
- 实现本地仓库、outbox、push/pull 和冲突处理；
- 为核心金额、日期和实体创建函数编写测试。

完成条件：核心 TypeScript 通过静态检查和单元测试。

### 阶段 C：业务页面

- 概览：跨模块统计和最近动态；
- 财务：账户、收入/支出、删除；
- 笔记：创建、编辑、搜索、置顶、删除；
- 英语：文章阅读、生词、掌握状态；
- 桌面侧栏和移动底部导航。

完成条件：桌面和 360 px 移动视口均无阻断操作。

### 阶段 D：PWA 与质量门禁

- manifest 和 Service Worker；
- 独立构建命令；
- CI 执行 typecheck、unit、桌面 Web build、PWA build；
- 校验 PWA 关键产物和 5 MB 体积门禁；
- 上传 `dist-web` artifact。

完成条件：PR 上所有 GitHub Actions 通过。

### 阶段 E：合入

- 检查 PR diff、未解决 review 和 CI；
- 仅在 head SHA 未变化且检查全部通过时 squash merge；
- 合入后检查 `main` workflow；
- 若主分支回归失败，优先修复；无法安全修复时 revert 合并提交。

## 5. 安全要求

1. 不在 localStorage/sessionStorage 保存密码、access token 或 refresh token；
2. API 默认 same-origin；生产环境由反向代理转发 `/api`；
3. 状态修改使用 CSRF token；
4. 所有用户数据缓存按 userId 隔离；
5. 退出清理用户缓存；
6. 文章正文以纯文本呈现，不直接注入不可信 HTML；
7. 金额仅接受最多两位小数并转换为安全整数分；
8. 不由客户端修改 `english.article`；
9. 冲突不静默覆盖服务器数据，记录本地冲突摘要；
10. Service Worker 不缓存 API 响应。

## 6. 测试策略

### 自动测试

- 金额解析边界；
- 实体工厂必填字段；
- 登录端点、Cookie 和 scope；
- 离线写入与重启恢复；
- push accepted + pull cursor；
- optimistic conflict；
- TypeScript 全量静态检查；
- 现有 Tauri Web 构建回归；
- 新 PWA 构建与产物检查。

### 手工联调

- 有效/无效登录；
- 登录刷新后恢复会话；
- 在线创建后桌面端可见；
- 桌面端修改后 Web pull 可见；
- 离线创建三类数据，刷新仍存在，恢复联网后同步；
- 两端同时修改同一实体，Web 显示冲突提示且采用服务器版本；
- 退出后刷新无法进入工作区；
- Chromium、Edge、Safari 移动视口；
- PWA 安装和离线启动。

详见 `docs/epic13/test-matrix.md`。

## 7. 性能预算

- `dist-web` 总体积小于 5 MB（CI 硬门禁）；
- 首屏不加载富文本编辑器、Excel、二维码等桌面端重依赖；
- 首屏仅一次 session 请求，业务同步在会话恢复后执行；
- 目标生产环境 LCP ≤ 3 秒（正常 4G、冷缓存）；
- API 与静态资源启用 HTTPS、HTTP/2/3、Brotli/Gzip 和长期 hash 缓存。

## 8. 风险与缓解

| 风险 | 缓解 |
|---|---|
| 浏览器 Cookie 跨域失败 | 生产采用同源 `/api` 反向代理；开发由 Vite proxy 转发 |
| 离线修改后版本冲突 | 严格使用 `baseServerVersion`，冲突采用服务器实体并可见提示 |
| localStorage 容量不足 | 本 EPIC 只缓存核心文本数据；附件和大正文离线缓存后续迁移 IndexedDB |
| 首次全量 pull 过大 | 每批 500、最多 25 批；超限提示再次同步；后续可接 snapshot |
| Service Worker 旧版本滞留 | cache name 版本化，activate 删除旧缓存 |
| Web 改动破坏桌面端 | `npm test` 同时构建 Tauri Web 和 PWA |

## 9. Definition of Done

- [x] 需求与后端契约完成核对；
- [x] Web/PWA 工程和业务功能实现；
- [x] 离线同步和冲突策略实现；
- [x] 自动测试和 CI workflow 加入；
- [x] 部署与测试文档加入；
- [x] 本地核心 TypeScript 编译和可执行冒烟测试通过；
- [ ] GitHub Actions 全部通过；
- [ ] PR 无未解决阻断项；
- [ ] 合入 `main`；
- [ ] `main` 合入后检查通过。
