# LifeTrace Desktop Frontend V2 Clean-room Rewrite

> 状态：V2 / 执行规范  
> 适用范围：`apps/desktop` 的 UI / renderer 层  
> 配套规范：`docs/ui/web-redesign.md`  
> 分支：`feature/frontend-v2-clean-rewrite`

---

## 1. 目标

LifeTrace Desktop V2 与 Web V2 同期重构，目标不是继续修补现有桌面界面，而是重建一套统一、现代、长期可维护的 LifeTrace Personal OS 前端体系。

Desktop 本轮采用：

> **UI Clean-room Rewrite + Native Capability Preservation**

即：

- 桌面视觉、页面结构、App Shell、组件体系、布局体系从零重写；
- 不允许旧 CSS、旧布局、旧组件结构影响 V2；
- 保留 Tauri 原生层、本地数据库、文件系统、Updater、Dialog、Process 等桌面能力；
- Web 与 Desktop 使用同一套 Design System、业务组件和信息架构；
- Desktop 在共享体验上进一步增加桌面级交互，而不是简单把 Web 页面塞进 Tauri WebView。

核心产品定位：

> LifeTrace Desktop 是 LifeTrace Personal OS 的桌面工作台，是 Web 的桌面增强形态，而不是另一套独立设计。

---

## 2. 为什么桌面端也必须 clean-room

当前 Desktop 已存在较多历史样式和页面级 CSS，继续增量改造容易导致：

- 页面之间存在不同年代的设计语言；
- CSS override 叠加；
- 同一业务在 Web / Desktop 出现两套组件；
- 新 Design Token 无法成为唯一事实来源；
- Agent 为了兼容旧 DOM 结构而妥协新布局；
- 重构后仍保留“旧软件换皮”的感觉。

因此 Desktop V2 不允许采用“继续在旧页面上换 CSS”的方式。

---

## 3. 重写边界

### 3.1 必须重写的 UI 层

Codex 在执行前必须先建立 Desktop 功能 inventory，再确定实际删除边界。

以下内容原则上属于旧 UI 实现，应被删除并从 V2 重建：

```text
apps/desktop/app/                 # 旧页面、旧 CSS、旧静态 UI
apps/desktop/src/components/      # 旧视觉组件，需逐项确认后清空重建
apps/desktop/src/ui/              # 旧 UI abstraction，需逐项确认后清空重建
apps/desktop/tauri-ui/            # 若属于旧 renderer / UI，纳入 clean-room
旧 renderer 入口
旧页面级样式
旧 layout / shell
旧 navigation UI
旧 theme / token 实现
```

禁止机械执行 `rm -rf apps/desktop/src`，因为该目录当前还包含 db、services、stores、types、lib 等可能承载业务和平台能力的代码。

### 3.2 默认必须保留

以下内容默认不是 UI clean-room 删除对象：

```text
apps/desktop/src-tauri/
apps/desktop/src/db/
apps/desktop/src/services/
apps/desktop/src/stores/
apps/desktop/src/types/
apps/desktop/src/lib/
apps/desktop/src/utils/
Tauri 配置
Updater 配置
本地数据库 schema / migration
文件系统逻辑
导入导出逻辑
本地 AI / 本地服务能力
同步协议
认证协议
平台 bridge
```

如果某个文件同时混合 UI 与业务逻辑，应先抽取业务逻辑，再删除旧 UI。

### 3.3 package.json 不直接删除

Desktop 已经使用 Tauri 2、React 19、Vite、Zustand、TipTap 等能力。V2 应审查依赖并整理，但不能像 Web clean-room 那样默认删除整个 `package.json`，因为其中包含桌面构建和原生能力依赖。

允许：

- 删除仅服务旧 UI 的依赖；
- 增加 V2 Design System / router / test 依赖；
- 调整 scripts；
- 统一 workspace dependency。

禁止在没有完成 dependency inventory 前重建 package.json。

---

## 4. Clean-room 强制规则

Desktop V2 开始后，Agent 不得将旧 Desktop UI 作为实现参考。

禁止：

```text
git show <old-ref>:apps/desktop/app/...
git show <old-ref>:apps/desktop/src/components/...
git diff <old-ref> -- apps/desktop/app
git log -p -- apps/desktop/app
git blame apps/desktop/app/...
```

也不得：

- 复制旧 CSS；
- 复制旧 DOM / JSX 页面结构；
- 为兼容旧 className 保留历史 class；
- 把旧页面截图作为 V2 视觉参考；
- 将旧组件改名后继续当 V2 组件使用；
- 因为某页面“已经能用”就跳过重新设计。

但允许读取旧代码中的 **非 UI 事实**，前提是这是确认桌面能力所必需，例如：

- Tauri command 名称；
- local database method；
- 文件导入导出入口；
- updater 调用；
- native event；
- platform capability；
- 数据类型和业务约束。

如果旧文件同时包含 UI 和平台调用，Agent 只能提取平台 contract，不得沿用其页面结构。

---

## 5. Web + Desktop 的统一架构

V2 不允许再次形成两套相互漂移的前端。

推荐形成共享 packages：

```text
apps/
├── web/
└── desktop/

packages/
├── design-system/
│   ├── tokens/
│   ├── components/
│   ├── icons/
│   └── styles/
├── frontend-core/
│   ├── domain/
│   ├── hooks/
│   ├── schemas/
│   └── utils/
├── frontend-features/
│   ├── today/
│   ├── plan/
│   ├── calendar/
│   ├── habits/
│   ├── fitness/
│   ├── finance/
│   ├── reading/
│   ├── notes/
│   └── review/
└── platform/
    ├── web/
    └── desktop/
```

目录名称可根据现有 monorepo 约束调整，但职责必须保持。

### 5.1 共享层负责

```text
Design Tokens
Primitive Components
Feature Components
Domain Types
Validation Schema
Formatting
Business View Models
Feature-level hooks
通用状态
页面主体内容
```

### 5.2 平台层负责

Web adapter：

```text
HTTP / Cloud API
Web Storage
Browser navigation
PWA capability
```

Desktop adapter：

```text
Tauri invoke
Local DB
Filesystem
Native Dialog
Updater
Process
Desktop notification
Window state
Native shortcuts
```

Feature 不应直接到处调用 `invoke()` 或浏览器 API，应通过 platform interface / adapter 隔离。

---

## 6. 页面共享策略

目标不是 100% 像素一致，而是 **同一个产品、同一种设计语言、不同平台优化**。

### 6.1 应高度共享

```text
Today
Habits
Fitness / Health
Finance
Review
Settings 内容区
通用表格
通用图表
表单
状态组件
空状态
错误状态
Loading / Skeleton
```

### 6.2 Desktop 应有专门形态

```text
Notes editor
Reading workspace
Plan / task detail
Calendar
Search
Quick capture
Import / Export
Local tools
File-backed workflows
```

Desktop 可以使用更高信息密度：

```text
Sidebar | List | Detail
Sidebar | Content | Inspector
Navigation | Editor | Context Panel
```

但必须来自统一 Design System。

---

## 7. Desktop App Shell

Desktop 应比 Web 更像真正的桌面生产力软件。

推荐：

```text
┌──────────────────────────────────────────────────────────┐
│ Native / custom title area                               │
├────────────┬─────────────────────────────────────────────┤
│ Sidebar    │ Workspace                                   │
│            │                                             │
│ Today      │ Page toolbar                                │
│ Plan       │ ─────────────────────────────────────────── │
│ Calendar   │ Main content                                │
│ Habits     │                                             │
│ Fitness    │                                             │
│ Finance    │                                             │
│ Reading    │                                             │
│ Notes      │                                             │
│ Review     │                                             │
│            │                                             │
│ Settings   │                                             │
└────────────┴─────────────────────────────────────────────┘
```

要求：

- Sidebar 可折叠；
- 适配窄窗口；
- 支持键盘导航；
- 支持 `Ctrl/Cmd + K` Command Palette；
- 支持 Quick Capture；
- 窗口状态恢复；
- 页面 toolbar 稳定；
- 不使用传统 Admin breadcrumb 堆叠；
- 不制作大面积 Dashboard card wall。

---

## 8. Desktop 特有交互

V2 应把 Desktop 的优势真正使用起来。

### 8.1 Keyboard First

至少设计：

```text
Ctrl/Cmd + K    Global Command
Ctrl/Cmd + N    Context-aware Quick Create
Ctrl/Cmd + F    Search current workspace
Ctrl/Cmd + ,    Settings
Esc             Close transient UI
```

具体快捷键需避免与编辑器 / OS 冲突。

### 8.2 Context Menu

对合理对象支持右键操作：

```text
Task
Note
Workout entry
Finance record
Reading item
File / attachment
```

禁止为“桌面感”到处添加无价值菜单。

### 8.3 Drag & Drop

适合时支持：

- 文件导入；
- 图片附件；
- 列表排序；
- Kanban / Task reorder；
- 将本地文件拖入 Notes / Reading。

### 8.4 Window-aware Layout

不要只按 mobile / desktop breakpoint 思考。

Desktop 必须处理：

```text
minimum supported width
narrow window
normal window
wide window
maximized
high DPI
```

窄窗口应自动收起辅助 panel，而不是产生横向滚动。

---

## 9. Design System

Desktop 与 Web 必须共用 V2 Design System。

基础技术建议继续沿用 Web V2 方向：

```text
React 19
TypeScript
Tailwind CSS
shadcn/ui pattern
Lucide
Recharts
Zustand only where appropriate
```

Desktop 已有 TipTap，编辑器能力可保留，但 Editor chrome、toolbar、bubble menu、side panel 必须按 V2 重新设计。

### 9.1 视觉原则

```text
Linear / Raycast / Notion / Vercel 的成熟生产力软件气质
neutral-first
低饱和
清晰 typography
克制 border
极少重阴影
适度 radius
高信息密度
少量 motion
```

### 9.2 禁止

```text
旧 CSS migration
CSS override chain
page-specific visual system
巨大 Hero
彩色渐变 Dashboard
Glassmorphism 全站
KPI card wall
每个 section 都套 Card
Emoji 主导航
过度动画
```

---

## 10. Finance

与 Web V2 相同：Finance 是业务实现复用的特例。

优先级：

```text
BeeCount Web 业务交互 / 成熟实现
        ↓
抽取可共享 Finance Feature
        ↓
适配 LifeTrace V2 Design Tokens
        ↓
Web / Desktop 共用
```

禁止 Desktop 再单独实现第三套 Finance UI。

---

## 11. Desktop 本地能力不能丢失

UI clean-room rewrite 完成后必须保证旧桌面应用已经具备、且仍属于产品需求的本地能力继续可用。

Codex 在删除 UI 前必须先建立 **Desktop Capability Inventory**，至少调查：

```text
Tauri commands
Local DB
Import / Export
Local files
QR / image handling
Update mechanism
Native dialogs
Process / local service
Offline behavior
Sync behavior
Authentication
Local-only tools
Editor-specific persistence
Window state
```

Inventory 的事实来源允许是现有非视觉逻辑代码和 `src-tauri`。

每个能力标记：

```text
KEEP
ADAPT
REMOVE（必须有明确产品理由）
```

没有 inventory，不允许先删整个 Desktop renderer。

---

## 12. 推荐实施顺序

### Phase 0 — Safety

1. 确认工作分支；
2. 确认 `main` 可恢复；
3. 更新 `AGENTS.md` clean-room 规则；
4. 不修改 main。

### Phase 1 — Inventory

只抽取事实，不设计：

```text
业务功能
Tauri capabilities
Local DB capability
Cloud capability
Desktop-only functions
Shared Web/Desktop functions
Existing tests
```

输出 capability matrix。

### Phase 2 — Boundary Extraction

将散落在旧 UI 内的：

```text
Tauri invoke
DB method
platform API
sync logic
data transform
```

抽取为可保留 platform / domain 层。

这一步结束后，旧 UI 应可以被安全删除而不损失底层能力。

### Phase 3 — Remove Legacy UI

形成独立提交，例如：

```text
chore(desktop): remove legacy UI for v2 clean rewrite
```

此后禁止从 Git history 恢复旧 UI。

### Phase 4 — Shared Foundation

先实现：

```text
Design Tokens
Primitive Components
Theme
Typography
Icon policy
App Shell
Platform adapters
Shared feature skeleton
```

### Phase 5 — Desktop Shell

实现：

```text
Desktop navigation
Window-aware layout
Command palette
Keyboard shortcuts
Quick capture
Native title/window behavior
```

### Phase 6 — Shared Features

按与 Web 相同的 feature contract 实现：

```text
Today
Plan
Calendar
Habits
Fitness / Health
Finance
Reading / English
Notes
Review
Search
Settings
```

避免 Web Agent 和 Desktop Agent 各自重新设计同一业务。

### Phase 7 — Desktop Enhancements

实现：

```text
Multi-pane layouts
Context menus
Drag & drop
Native dialogs
Offline/local behavior
File workflows
Editor enhancements
Window persistence
```

### Phase 8 — Verification

完成测试、视觉一致性检查、桌面平台能力回归。

---

## 13. 测试门禁

Desktop V2 未达到以下条件不得合并：

```text
TypeScript typecheck PASS
Unit tests PASS
Web renderer build PASS
Browser-compatible build（如项目仍保留）PASS
Rust tests PASS
Tauri build PASS
Critical desktop smoke tests PASS
Core local capabilities PASS
Cloud sync/auth PASS
Import/export PASS
No legacy UI CSS imported PASS
No old Desktop visual components restored PASS
Web/Desktop Design System consistency PASS
```

### 13.1 必测桌面路径

至少覆盖：

```text
启动应用
登录 / session restore
Sidebar navigation
Command palette
创建 / 编辑核心业务数据
Notes editor
Reading workflow
Finance
Local persistence
Cloud sync
File import/export
Native dialog
Updater path（可 mock）
Restart persistence
Window resize
Narrow window
```

---

## 14. Web 与 Desktop 的执行关系

不建议先完全做完 Web，再复制到 Desktop。

更合理：

```text
Shared Design System
        ↓
Shared App / Feature contracts
        ↓
┌──────────────────┬──────────────────┐
│ Web Shell        │ Desktop Shell    │
│ Web adapters     │ Tauri adapters   │
└──────────────────┴──────────────────┘
        ↓
Shared Features
        ↓
Platform-specific enhancement
```

每完成一个核心 feature，应同时验证：

- Web 是否符合 responsive 要求；
- Desktop 是否符合 keyboard / multi-pane / window-aware 要求；
- 共享组件是否没有平台耦合；
- 平台差异是否通过 adapter 处理。

---

## 15. Git 提交建议

```text
chore(frontend): establish V2 clean-room rules

docs(desktop): inventory desktop capabilities

refactor(desktop): extract native and domain boundaries

chore(web): remove legacy frontend
chore(desktop): remove legacy UI

feat(frontend): create shared design system
feat(frontend): create shared feature architecture

feat(web): implement V2 app shell
feat(desktop): implement V2 desktop shell

feat(frontend): implement today
feat(frontend): implement planning and calendar
feat(frontend): implement habits
feat(frontend): implement fitness and health
feat(frontend): integrate BeeCount finance
feat(frontend): implement reading and notes
feat(frontend): implement review and search

feat(desktop): add native productivity interactions

test(frontend): complete V2 regression suite

docs(frontend): finalize V2 architecture
```

---

## 16. Definition of Done

Desktop V2 只有同时满足以下条件才算完成：

1. 旧 Desktop UI 与历史 CSS 不再进入生产构建；
2. V2 Desktop 未从 Git 历史恢复旧 UI；
3. Tauri / Local DB / File / Sync 等应保留能力仍工作；
4. Desktop 与 Web 使用同一 Design System；
5. 主要业务 feature 不存在两套独立视觉实现；
6. Desktop 有真正的 keyboard-first / window-aware / native-enhanced 体验；
7. 没有旧 CSS override chain；
8. lint / typecheck / unit / Rust / build / critical smoke tests 全部通过；
9. 文档与实现一致；
10. 经过最终回归后才允许合并 `main`。

---

# 最终原则

> **Web V2 与 Desktop V2 是同一个 LifeTrace 产品的两种运行形态。**

Web 负责跨设备与响应式访问；Desktop 在共享产品体验的基础上提供本地数据、文件、快捷键、多面板和 Tauri 原生能力。

本轮目标不是“把两个旧前端分别变好看”，而是结束 Web / Desktop 各自演化的历史，让它们共同建立在一套 V2 Design System、Feature Architecture 和业务模型之上。