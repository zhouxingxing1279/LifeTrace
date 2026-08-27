# Desktop 去 Web 化实施方案

> 状态：执行中
> 基线：`main@80fbfbbc0c42f95a1580e960a2a94f4b4fa1e9f2`
> 执行分支：`refactor/desktop-decouple-web`
> 创建日期：2026-08-27

## 1. 背景与问题定义

当前 LifeTrace 的 `apps/desktop` 和 `apps/web` 名义上是两个客户端，但 Desktop 在线工作区直接依赖 Web 内部实现：

- `apps/desktop/tauri-ui/main.tsx` 加载 `apps/web/src/styles/globals.css`；
- `DesktopCloudWorkspace.tsx` 直接导入 Web 的 `AppContext`、`DesktopFeatureRouter`、`CloudDataStore`；
- `apps/web/src/app/DesktopFeatureRouter.tsx` 为 Desktop 直接加载 Web Feature Pages；
- `vite.tauri.config.ts` 使用 Web 的 PostCSS 配置并允许读取整个 `apps/`；
- `ensure-shared-web-deps.mjs` 会为 Desktop 构建安装 `apps/web/node_modules`；
- 在线写操作以 `CloudDataStore -> Cloud API -> syncLocalReplica()` 为主路径，而不是 `SQLite -> Outbox -> Sync`。

这导致 Desktop 实际接近“Web Cloud Client + Tauri Shell”，削弱了 Local-first、离线写入、后台任务、快捷键、文件系统、通知、托盘、多窗口等桌面能力。

## 2. 目标架构

最终架构必须满足：

```text
                 Shared Packages / Contracts
                    /       |       \
                   /        |        \
              contracts   domain   design/ui primitives
                  ^          ^          ^
                  |          |          |
          +-------+----------+----------+-------+
          |                                  |
      Desktop                              Web
          |                                  |
  Desktop Runtime                       Web Runtime
  Desktop Router                        Web Router
  Desktop Pages                         Web Pages
          |                                  |
   Repositories                        Cloud Client
          |                                  |
      SQLite                            HTTPS API
          |
      Outbox
          |
   Background Sync
          |
        Cloud
```

硬规则：

1. `apps/desktop/**` 不得直接 import `apps/web/**`；
2. `apps/web/**` 不得直接 import `apps/desktop/**`；
3. Desktop 核心业务写入必须先落本地 SQLite；
4. Cloud 同步属于后台副作用，不是 Desktop CRUD 成功的前置条件；
5. 两端共享 Domain/Contracts，不共享 App Runtime、Router、Page、Store、Shell；
6. Desktop 必须能够在 `apps/web/node_modules` 不存在时独立 install/build/test。

## 3. 非目标

本次重构不做以下事情：

- 不删除或重写 Tauri Rust Backend；
- 不重做已有 SQLite Migration；
- 不改变 Cloud Sync Protocol 的外部契约；
- 不删除 Vault、Encrypted Album、Photo Sync、Updater、Window State、Profile Isolation 等 Desktop 基础设施；
- 不同步进行 Web 大规模 UI 重构；
- 不为了“代码复用率”把 Desktop 页面重新抽回 Web。

## 4. 共享边界

### 4.1 允许共享

- Contracts / DTO / Schema；
- Entity registry；
- validation；
- 纯业务算法；
- 财务/习惯/复习等纯计算规则；
- Sync schema / protocol；
- 格式化工具；
- Design Tokens；
- 无平台状态、无数据源耦合的 UI primitive。

### 4.2 禁止共享

- AppContext / Runtime Provider；
- Router；
- Page / Workspace；
- App Shell；
- 平台 Store；
- CloudDataStore；
- Session Runtime；
- 文件、窗口、通知、快捷键等平台集成实现。

## 5. 迁移策略

采用 Strangler Pattern（绞杀式迁移），而不是一次性重写。

每个模块按以下顺序切换：

```text
旧：Desktop -> Web Feature -> CloudDataStore

过渡：Desktop Router -> Desktop Feature Adapter -> Desktop Repository

新：Desktop Page -> Desktop UseCase -> SQLite Repository -> Outbox -> Sync
```

旧路径只在对应模块尚未迁移时保留；每完成一个模块即删除相应 allowlist 和跨 App import。

## 6. 执行阶段

### Phase 0：基线与架构门禁

目标：冻结耦合面，禁止新增 Desktop↔Web 跨 App 依赖。

任务：

- [ ] 增加架构边界检查脚本；
- [ ] 记录当前允许的历史跨 App 依赖 allowlist；
- [ ] CI/测试执行边界检查；
- [ ] 新增依赖不在 allowlist 时直接失败；
- [ ] 每消除一项历史依赖同步收缩 allowlist。

验收：后续提交不能扩大 Desktop→Web 或 Web→Desktop 耦合面。

### Phase 1：Desktop 构建链解耦

目标：Desktop 构建不再依赖 Web 工程依赖安装过程。

任务：

- [ ] 移除 `ensure-shared-web-deps.mjs`；
- [ ] 移除 `prepare:web-shared` 生命周期；
- [ ] Desktop 使用自己的 PostCSS/CSS 配置；
- [ ] 缩小 Vite `fs.allow`；
- [ ] Desktop 不再依赖 `apps/web/node_modules`；
- [ ] 在页面迁移完成前，可暂时保留有限源码读取，但必须由 Phase 0 allowlist 管控。

验收：Desktop npm 生命周期不会执行 `npm install --prefix apps/web`。

### Phase 2：独立 Desktop Runtime

目标：Desktop UI 不再依赖 Web `AppContext`。

新增建议：

```text
apps/desktop/src/app/
├── DesktopRuntime.tsx
├── DesktopProviders.tsx
└── runtimeTypes.ts
```

Runtime 至少暴露：

- 当前 profile / identity；
- online / sync status；
- privacy / theme；
- repositories；
- desktop commands；
- platform capabilities。

验收：Desktop 页面不再需要 Web `AppRuntimeProvider`。

### Phase 3：Repository 与 Local-first 写路径

目标：Desktop 的 Source of Truth 恢复为 SQLite。

建议结构：

```text
apps/desktop/src/data/
├── repositories/
│   ├── financeRepository.ts
│   ├── habitsRepository.ts
│   ├── notesRepository.ts
│   ├── englishRepository.ts
│   ├── executionRepository.ts
│   ├── fitnessRepository.ts
│   └── preferencesRepository.ts
├── sqlite/
├── outbox/
└── sync/
```

标准写路径：

```text
UI -> UseCase -> Repository -> SQLite transaction
                           ├-> entity update
                           └-> outbox record
                                     |
                                  commit
                                     |
                              UI immediately succeeds
                                     |
                              background sync
```

验收：断网时核心 CRUD 正常工作并可在重启后读取；恢复网络后自动同步。

### Phase 4：独立 Desktop Router

目标：删除 Web 内的 Desktop 路由职责。

新增：

`apps/desktop/src/app/DesktopRouter.tsx`

迁移完成后删除：

`apps/web/src/app/DesktopFeatureRouter.tsx`

验收：Desktop Router 只加载 `apps/desktop/**` 或正式共享包中的组件。

### Phase 5：按模块迁移页面

优先级：

| 优先级 | 模块 | 原因 |
| --- | --- | --- |
| P0 | Today | Desktop 主入口 |
| P0 | Execution / Calendar | 通知、后台、快捷键价值高 |
| P0 | Notes | 本地搜索、文件、快捷记录价值高 |
| P0 | Finance | Local-first 写入要求高 |
| P1 | Habits | 通知/托盘/快速打卡 |
| P1 | Fitness | 已有 Desktop Import |
| P1 | English | 音频、本地数据、快捷操作 |
| P1 | Search | 可演进 SQLite FTS5 |
| P2 | Health / Review | 上层聚合数据 |
| P2 | Assistant | 后续连接本地工具/本地 AI |
| P2 | Settings | 最终统一 Desktop Settings |

每个模块完成标准：

- [ ] Desktop Page 位于 `apps/desktop`；
- [ ] 不依赖 Web Feature；
- [ ] 本地数据可读写；
- [ ] 离线可用；
- [ ] 同步后 Cloud 一致；
- [ ] Web 原页面无回归。

### Phase 6：样式完全解耦

目标：Desktop 不再加载 Web globals/Tailwind visual contract。

建议：

```text
apps/desktop/src/styles/
├── tokens.css
├── reset.css
├── desktop-shell.css
├── components.css
└── features/
```

Design Token 可下沉共享，但页面 CSS 不跨 App 引用。

验收：`apps/desktop` 不引用 `apps/web/src/styles/**`。

### Phase 7：删除 Web Bridge

删除/替代：

- [ ] `DesktopCloudWorkspace.tsx` 中的 Web Runtime Adapter；
- [ ] `apps/web/src/app/DesktopFeatureRouter.tsx`；
- [ ] 所有 `../../../web/src/**` import；
- [ ] Web CSS import；
- [ ] Web PostCSS dependency；
- [ ] 共享 Web node_modules bootstrap。

最终验收：

```bash
rg 'web/src|apps/web' apps/desktop
# 0 个架构依赖结果（文档/注释中允许明确的历史说明除外）

rg 'desktop/src|apps/desktop' apps/web
# 0 个架构依赖结果
```

### Phase 8：恢复/增强 Desktop 原生能力

在解耦完成后逐步增加：

- [ ] 系统托盘；
- [ ] Quick Capture；
- [ ] Quick Finance；
- [ ] 全局快捷键；
- [ ] Windows Notification；
- [ ] File Drop / 文件导入；
- [ ] SQLite FTS5；
- [ ] Background Jobs；
- [ ] Mini Window / 多窗口；
- [ ] 文件关联 / Deep Link；
- [ ] Clipboard Integration。

## 7. 关键文件改动清单

### 保留并继续使用

- `apps/desktop/src-tauri/**`；
- `apps/desktop/tauri-ui/apiBridge.ts`；
- `apps/desktop/tauri-ui/vaultBridge.ts`；
- `apps/desktop/tauri-ui/windowState.ts`；
- `apps/desktop/src/services/cloudAuth.ts`；
- `apps/desktop/src/services/cloudSync.ts`；
- `apps/desktop/src/services/appUpdater.ts`；
- `apps/desktop/src/services/clientObservability.ts`；
- Existing SQLite / profile / sync / vault / photo capabilities。

### 重写或拆分

- `apps/desktop/src/components/DesktopCloudWorkspace.tsx`；
- Desktop online/offline mode selection in `DesktopApp.tsx`；
- Desktop data access abstraction；
- Desktop routing and runtime。

### 最终删除

- `apps/web/src/app/DesktopFeatureRouter.tsx`；
- `apps/desktop/scripts/ensure-shared-web-deps.mjs`；
- Desktop 对 Web globals/PostCSS/node_modules 的依赖。

## 8. 测试矩阵

### 8.1 架构

- Desktop→Web 新增依赖必须失败；
- Web→Desktop 新增依赖必须失败；
- 最终历史 allowlist 清零。

### 8.2 Local-first

断开 Cloud/网络后验证：

- 创建/修改/删除笔记；
- 创建账单；
- 创建/完成任务；
- 习惯打卡；
- 英语学习记录；
- 关闭并重启 Desktop，数据仍存在；
- 恢复网络后 Outbox 自动同步。

### 8.3 Desktop 回归

- 登录/自动恢复；
- 离线启动；
- Profile isolation；
- Sync push/pull/conflict；
- Vault；
- Photo Sync；
- Fitness Import；
- Window placement；
- Updater；
- Logout。

### 8.4 Web 回归

- typecheck；
- unit test；
- build；
- Playwright E2E。

## 9. 回滚策略

每个 Feature 单独迁移和提交，不进行一次性全量页面替换。

单模块迁移失败时，只回滚该模块到旧 Web-backed route，不回滚已稳定完成的 Runtime、Repository 或其他模块。

禁止在同一个不可分割提交中同时：

1. 重写全部 Desktop UI；
2. 改 Sync Protocol；
3. 改数据库 Schema；
4. 删除所有旧路径。

## 10. 提交建议

按以下提交组执行：

1. `docs: add desktop web decoupling implementation plan`
2. `chore: guard cross-app dependencies`
3. `build(desktop): remove web dependency bootstrap`
4. `refactor(desktop): introduce native runtime`
5. `refactor(desktop): add local repository layer`
6. `refactor(desktop): add local-first outbox writes`
7. `refactor(desktop): own desktop router`
8. 按模块迁移提交
9. `refactor(desktop): remove web runtime bridge`
10. `build(desktop): fully detach web styles and tooling`
11. `test: complete desktop local-first regression`

## 11. Definition of Done

全部满足才允许合并 `main`：

- [ ] `apps/desktop` 不直接 import `apps/web`；
- [ ] `apps/web` 不包含 Desktop Router；
- [ ] Desktop 不依赖 Web node_modules；
- [ ] Desktop 不使用 Web PostCSS；
- [ ] Desktop 不加载 Web globals.css；
- [ ] Desktop Router 位于 `apps/desktop`；
- [ ] Desktop Runtime 独立；
- [ ] Desktop 核心页面位于 `apps/desktop`；
- [ ] 核心写操作首先落 SQLite；
- [ ] 无网络可完成核心 CRUD；
- [ ] Outbox 恢复网络后同步成功；
- [ ] Tauri 原有能力无回归；
- [ ] Web 无功能回归；
- [ ] Desktop 可以独立 install/build/test；
- [ ] 架构 allowlist 清零。

## 12. 当前执行记录

- 2026-08-27：建立实施文档；创建 `refactor/desktop-decouple-web` 分支；开始 Phase 0。
