# Web UI 完全重构实施方案

## 1. 文档目的

本文档定义 LifeTrace **Web 端完全推倒重构**的目标架构、设计参考、页面规划、财务模块复用方式、实施顺序、测试门禁和最终切换标准，作为后续 Agent 执行 Web 重构时的权威依据。

本次不是在现有 `apps/desktop/web-client` 上继续增量换肤，也不是继续叠加 `web-*.css`。目标是重新建立独立、长期可维护的 `apps/web`，新版本完成全部功能对账和测试后切换入口，再删除旧 Web 实现。

核心定位：

> LifeTrace Web 是 Personal OS / Personal Dashboard，不是传统企业 Admin 后台。

硬性目标：

- 首页围绕“今天、趋势、下一步行动”组织；
- Desktop / Tablet / Mobile 均为一等平台；
- 使用统一 Design System 和正式 React UI Component；
- 非财务模块必须参考本文指定的前端模板体系进行重新设计；
- 财务栏目必须源码级复用 BeeCount Cloud Web，而不是重新仿写；
- 旧 Web 仅作为功能、数据契约和回归基线，不作为新视觉实现的地基。

---

# 2. 设计参考：必须使用最初选定的模板体系

本次重构的视觉与页面设计，必须以最初筛选的以下前端模板 / UI 体系为参考库：

1. **shadcn/ui**
2. **Tremor**
3. **Preline UI**
4. **Tailwind Plus / Catalyst**
5. **Aceternity UI**
6. **Magic UI**
7. **Shadcnblocks**
8. **TailAdmin**

这些参考不是要求把 8 套库全部安装进项目，而是作为 **页面结构、交互模式、信息层级、组件设计与动效设计的参考样本**。

禁止只写一句“Linear / Vercel 风格”然后凭感觉实现。每个核心页面在编码前都必须能指出主要参考了哪一套模板的哪类页面模式。

## 2.1 各参考源的职责

| 参考源 | 主要参考内容 | 在 LifeTrace 中的角色 |
|---|---|---|
| shadcn/ui | Button、Dialog、Sheet、Tabs、Command、Table、Form、Card、Token | **基础组件规范，最高优先级** |
| Catalyst | Sidebar、Application Shell、Settings、List、Table、Detail Layout | **全局布局与成熟应用感** |
| Shadcnblocks | Dashboard、Settings、Todo、Kanban、Calendar、完整页面组合 | **页面级结构模板，避免从空白画页面** |
| Tremor | KPI、Trend、Area/Bar/Donut、Analytics Dashboard | **数据可视化和分析页面** |
| Preline UI | Dashboard、Profile、Account、Settings、Form、响应式页面 | **完整页面和响应式参考** |
| Magic UI | Number Ticker、轻量进入动画、完成反馈 | **局部微交互** |
| Aceternity UI | 高级 Hover、AI 区域、强调卡片、特殊背景 | **AI / 特殊入口局部增强** |
| TailAdmin | 高密度 Dashboard、Data Table、管理型信息组织 | **复杂表格/高密度页面的备用参考** |

## 2.2 参考优先级

发生设计冲突时按以下优先级决策：

```text
业务可用性
  > shadcn/ui 组件一致性
  > Catalyst 应用布局
  > Shadcnblocks / Preline 页面结构
  > Tremor 数据表达
  > TailAdmin 高密度信息组织
  > Magic UI / Aceternity 装饰与动效
```

Magic UI / Aceternity 永远不能为了视觉效果破坏可读性、性能或交互一致性。

## 2.3 页面必须建立“参考映射”

实现每个核心页面前，在任务记录或 PR 描述中写清：

```text
Page: /app/today
Primary reference: Shadcnblocks Dashboard
Layout reference: Catalyst Application Shell
Data reference: Tremor
Interaction reference: shadcn/ui
Special motion: Magic UI Number Ticker（如需要）
```

这不是要求逐像素复制，而是防止 Agent 自由发挥导致全站页面风格漂移。

---

# 3. 页面与模板参考映射

## 3.1 App Shell

主要参考：

```text
Catalyst
+ shadcn/ui Sidebar / Command
+ Preline Dashboard Shell
```

目标：

- 220–248px Desktop Sidebar；
- 支持 collapsed；
- 顶部 Header 克制，不做大 Hero；
- `Ctrl/Cmd + K` 打开全局 Command Palette；
- Desktop 使用 Sidebar，Mobile 使用 Bottom Navigation + More Sheet；
- 页面标题、Breadcrumb、Page Action 形成稳定模板。

不要沿用旧 `.hx-shell`。

## 3.2 今日 / Dashboard

主要参考：

```text
Shadcnblocks Dashboard
+ Tremor Analytics
+ Catalyst Dashboard composition
```

局部可参考：

```text
Magic UI Number Ticker
Aceternity 的轻量重点卡片
```

首页只回答五个问题：

1. 今天最重要的是什么；
2. 已完成多少；
3. 下一步做什么；
4. 最近趋势是否异常；
5. 是否需要进入某个业务模块。

推荐结构：

```text
Greeting / Date / Quick Add

Today Focus
┌──────────────────────────────────────────────┐
│ 今日主要目标 / 完成度 / 下一步              │
└──────────────────────────────────────────────┘

Today Actions                 Schedule
┌─────────────────────────┐   ┌──────────────────┐
│ Tasks + Habits          │   │ Time blocks      │
└─────────────────────────┘   └──────────────────┘

Key Trends
┌──────────────────────────────────────────────┐
│ Habit / Workout / Learning / Finance         │
└──────────────────────────────────────────────┘

Recent Activity                  Quick Actions
```

首屏最多 4 个等权 KPI，不允许变成 KPI 墙。

## 3.3 计划与待办

主要参考：

```text
Shadcnblocks Todo / Project Management
+ Catalyst Lists / Detail Panels
+ Preline Task UI
```

结构：

```text
Inbox
Today
Upcoming
Projects
Completed
```

Desktop 推荐 List + Detail Drawer；Mobile 使用 List → Fullscreen Detail。

## 3.4 日历

主要参考：

```text
Shadcnblocks Calendar
+ Catalyst toolbar/filter patterns
+ Preline responsive layout
```

至少支持 Month / Week / Day / Agenda。Mobile 默认 Agenda / Day，不把桌面 Month 强行压缩。

## 3.5 坚持 / Habits

主要参考：

```text
Tremor KPI / Trend
+ Shadcnblocks Dashboard Card
```

核心：今日打卡、streak、7/30 天趋势、Heatmap、完成率。

## 3.6 Fitness / Health

主要参考：

```text
Tremor Analytics
+ Shadcnblocks Dashboard
+ Apple Health 的信息组织方式
```

Apple Health 仅作为健康信息层级参考，不作为本项目最初模板列表的替代。

推荐：

```text
This Week
Training Volume Trend
Recent Workouts
Muscle / Exercise Distribution
Body Metrics
```

## 3.7 笔记

主要参考：

```text
Catalyst three/two-pane application layout
+ shadcn/ui command/dialog/editor controls
+ Preline content workspace
```

Desktop：`Notes List | Editor`；需要文件夹时扩展为 `Folders | Notes | Editor`。

## 3.8 英语学习

主要参考：

```text
Catalyst content layout
+ shadcn/ui controls
+ Shadcnblocks dashboard for learning history
```

阅读器必须以内容为中心，进入阅读模式后弱化 App Shell。保留高亮、快捷笔记、已读状态、学习历史等功能。

## 3.9 复盘

主要参考：

```text
Shadcnblocks Dashboard
+ Tremor 7/30-day summaries
+ Catalyst form/content layout
```

## 3.10 Settings

主要参考：

```text
Catalyst Settings
+ shadcn/ui Form / Tabs / Switch / AlertDialog
+ Preline Account Settings
```

设置结构：

```text
Profile
Appearance
Cloud & Sync
Devices
Privacy
Security
Data
About
Danger Zone
```

## 3.11 Auth

主要参考：

```text
Preline Login / Account
+ shadcn/ui Form
+ Catalyst typography
```

不要制作营销型登录页。

## 3.12 AI 助手

主要参考：

```text
shadcn/ui Command / Dialog
+ Aceternity UI
+ Magic UI
```

Aceternity / Magic UI 只用于 AI 输入、状态变化和少量强调，不把整个产品改造成发光 SaaS 官网。

---

# 4. 已确认的架构决策

## 4.1 Web 前端完全推倒重构

允许：

- 重写 App Shell；
- 重写路由；
- 重写页面组件；
- 重写 Design System；
- 重写响应式布局；
- 重写 Dashboard；
- 重组目录；
- 将 Web 从 `apps/desktop/web-client` 迁出为独立 `apps/web`；
- 完成切换后删除旧 Web UI。

旧 Web 仅用于：

```text
功能清单
API 调用参考
业务行为回归
数据契约参考
```

不允许为了兼容旧 CSS 继续扩大历史包袱。

## 4.2 后端不是本轮推倒对象

默认继续复用：

- LifeTrace Cloud；
- 现有同步协议；
- 现有实体模型；
- 认证与会话；
- BeeCount 兼容接口；
- 现有业务数据。

UI 暴露真实接口缺口时可以补 API，但不无理由重做后端。

---

# 5. 新 Web 目标架构

推荐：

```text
apps/
├── desktop/
├── web/
│   ├── package.json
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── app/
│       ├── layouts/
│       ├── components/
│       │   ├── ui/
│       │   ├── data-display/
│       │   ├── feedback/
│       │   └── navigation/
│       ├── features/
│       │   ├── dashboard/
│       │   ├── execution/
│       │   ├── habits/
│       │   ├── fitness/
│       │   ├── health/
│       │   ├── finance/
│       │   │   ├── beecount/
│       │   │   └── adapters/
│       │   ├── notes/
│       │   ├── english/
│       │   ├── review/
│       │   ├── search/
│       │   └── settings/
│       ├── services/
│       ├── hooks/
│       ├── stores/
│       ├── lib/
│       └── styles/
└── photo-challenge-pwa/
```

重构期间允许双轨：

```text
apps/desktop/web-client/   # legacy，只做回归
apps/web/                  # new，唯一新开发目标
```

完成门禁后切换构建并删除 legacy。

---

# 6. 技术栈

新 Web：

```text
React 19
TypeScript
Vite
React Router
Tailwind CSS
shadcn/ui 模式组件
Lucide React
Recharts
Zustand（只用于必要客户端状态）
Vitest / 现有测试体系
```

BeeCount Cloud Web 当前使用 React + Vite + Tailwind + shadcn schema + Recharts，因此新 LifeTrace Web 的技术选择应优先保证财务源码复用成本低。

Tailwind 大版本升级不要与本次重构同时进行。先采用与导入 BeeCount Web 兼容的配置，完成切换后再单独升级。

路由统一使用 React Router，页面 lazy load。

建议路由：

```text
/
/login
/app/today
/app/execution
/app/calendar
/app/habits
/app/fitness
/app/health
/app/finance/*
/app/notes
/app/english/*
/app/review
/app/search
/app/settings/*
```

---

# 7. Design System

## 7.1 基础组件来源

新 UI Primitive 主要采用 shadcn/ui 的组织方式，而不是继续用 `.hx-*` / `.lt-*` 作为核心抽象。

至少包含：

```text
Button
Input / Textarea
Select
Checkbox / Switch
Badge
Card
Tabs
Table
Dialog / AlertDialog
Sheet / Drawer
DropdownMenu
Popover
Tooltip
Command
Skeleton
Separator
ScrollArea
Progress
Toast
```

## 7.2 Token

使用一套语义 Token：

```text
background
foreground
card
card-foreground
popover
popover-foreground
primary
primary-foreground
secondary
secondary-foreground
muted
muted-foreground
accent
accent-foreground
destructive
border
input
ring
success
warning
info
income
expense
chart-1 ... chart-n
```

非财务区域保留低饱和绿色作为 LifeTrace 品牌色，页面主体以 neutral 为主。

## 7.3 视觉约束

建议：

```text
Page title      24–28 / 600
Section title   18–20 / 600
Card title      14–16 / 600
Body            14 / 400
Secondary       12–13 / 400
Metric          24–32 / 600
```

圆角以 6 / 8 / 10 / 12 为主；普通信息卡主要依赖 border + surface 建立层级。

禁止：

- 大面积高饱和渐变；
- 每张 Card 都有大阴影；
- 20px+ 圆角滥用；
- 无意义 Glassmorphism；
- 每个模块自己发明组件规范；
- Emoji 替代正式图标；
- 所有内容都套 Card；
- 把 TailAdmin 的企业 Admin 感原样带入 Personal OS。

---

# 8. 财务模块：BeeCount Web 源码级复用

## 8.1 正确上游

财务 Web 源码来自：

```text
TNT-Likely/BeeCount-Cloud
frontend/apps/web/
frontend/packages/ui/
frontend/packages/web-features/
frontend/packages/api-client/
```

不是 `BeeCount-Website`。

## 8.2 复用优先级

```text
直接复用源码
  > Adapter 适配
  > 小范围视觉对齐
  > 重新实现
```

禁止看 BeeCount 截图后重新写一份“差不多”的财务页。

必须优先复用：

```text
OverviewPage
TransactionsPage
CalendarPage
LedgersPage
BudgetsPage
AccountsPage
CategoriesPage
TagsPage
ImportPage
```

后端能力满足时继续复用：

- Shared Ledger dialogs；
- Transaction edit dialogs；
- Entity dialogs；
- Category icon；
- Attachment cache；
- Page Data Cache；
- SyncSocket；
- AI Parse Transaction；
- 财务颜色偏好。

## 8.3 BeeCount 与最初模板体系的关系

BeeCount **决定财务模块的功能、领域交互和源码基线**；最初选定的前端模板体系仍决定 LifeTrace 的整体产品视觉方向。

因此：

```text
LifeTrace AppShell                 → Catalyst / shadcn / Preline
LifeTrace Dashboard               → Shadcnblocks / Tremor
LifeTrace Settings                → Catalyst / Preline / shadcn
Finance feature logic & pages     → BeeCount Web 源码
Finance outer integration         → LifeTrace AppShell
Finance global UI consistency     → LifeTrace Token + shadcn 基础规范
```

不能为了统一外观大幅改写 BeeCount 财务内部逻辑，也不能让 BeeCount 的全局 Shell 覆盖整个 LifeTrace。

## 8.4 集成结构

```text
LifeTrace AppShell
└── FinanceWorkspace
    ├── BeeCount Overview
    ├── BeeCount Transactions
    ├── BeeCount Calendar
    ├── BeeCount Ledgers
    ├── BeeCount Budgets
    ├── BeeCount Accounts
    ├── BeeCount Categories
    ├── BeeCount Tags
    └── BeeCount Import
```

不直接复制：

```text
BeeCount LoginPage
BeeCount 全局 AppShell
BeeCount 全局 AppHeader
BeeCount Admin Users / Backup / Cleanup
```

## 8.5 Finance Adapter

```text
features/finance/
├── beecount/
├── adapters/
│   ├── auth-adapter.ts
│   ├── ledger-adapter.ts
│   ├── transaction-adapter.ts
│   ├── account-adapter.ts
│   ├── category-adapter.ts
│   ├── tag-adapter.ts
│   ├── budget-adapter.ts
│   ├── import-adapter.ts
│   └── sync-adapter.ts
└── index.ts
```

BeeCount UI 尽量按原领域模型工作，Adapter 连接 LifeTrace 已实现的 BeeCount-compatible backend。

## 8.6 上游追踪和许可

新增：

```text
apps/web/src/features/finance/beecount/UPSTREAM.md
```

记录：

```text
Upstream repository
Upstream commit SHA
Imported paths
Local modifications
Intentionally omitted files
Sync procedure
License notice
```

必须保留 BeeCount Cloud 作者、版权和许可信息；第三方声明中登记 BeeCount Cloud。未来商业化前必须重新审查其商业许可要求。

---

# 9. 数据可视化

全站使用 Recharts 作为底层，数据视觉主要参考 Tremor。

统一封装：

```text
ChartCard
ChartTooltip
ChartLegend
TimeRangeSelector
TrendIndicator
MetricCard
```

规则：

- 不为不同 feature 引入不同图表库；
- 财务保留 BeeCount Recharts 实现；
- 非财务趋势也复用统一 primitive；
- Dark mode 保证可读；
- 空数据必须有 Empty State；
- 手机端减少 legend 和轴信息密度。

---

# 10. 响应式

统一断点：

```text
< 640px       Mobile
640–1023px    Tablet
>= 1024px     Desktop
>= 1440px     Wide
```

Mobile：

- 不显示桌面 Sidebar；
- Bottom Nav + More Sheet；
- Table 转 List/Card；
- Dialog 优先 Sheet / Fullscreen；
- 主要点击区域 >= 44px。

Tablet：Sidebar 可 collapsed，Detail 使用 Drawer。

Desktop：稳定 Sidebar，允许双/三栏工作区和高密度 Table。

---

# 11. 全局搜索 / Command Palette

主要参考 shadcn `Command`、Catalyst Search 和现代 command palette。

入口：Header Search + `Ctrl/Cmd + K`。

搜索范围：任务、项目、坚持、训练、财务交易、账本、笔记、英语、复盘和设置。

支持命令：

```text
新建任务
记录支出
开始训练
新建笔记
打开今日
打开财务
```

---

# 12. 主题与动效

主题：`system / light / dark`。

所有颜色必须来自 Token；BeeCount finance dark mode 和收入/支出颜色规则保留。

动效主要参考 Magic UI / Aceternity，但只允许：

- route fade / slide；
- number ticker；
- progress；
- dialog / drawer；
- command palette；
- 成就完成反馈；
- AI 状态反馈。

禁止常驻发光、粒子背景、无限浮动和每张卡片都 stagger animation。

必须支持 `prefers-reduced-motion`。

---

# 13. 实施阶段

## Phase 0：功能与设计参考盘点

先建立旧 Web 功能矩阵，同时建立页面设计参考矩阵。

每页记录：

```text
Route
Feature
Read / Write API
Permission
Desktop / Mobile behavior
Edge states
Replacement page
Primary template reference
Secondary template reference
Data visualization reference
```

没有完成这一步，不开始大规模编码。

## Phase 1：建立 `apps/web`

完成 Vite、React Router、Tailwind、shadcn-compatible UI、Token、Lint/Test/Build、Error Boundary、API Base。

同时建立可运行的 Story / Showcase 页面，集中展示 Button、Card、Form、Table、Dialog、Sheet、Navigation、Metric 和 Chart，先确认视觉基线。

## Phase 2：App Shell + Auth

按 Catalyst + shadcn + Preline 参考重写 App Shell、Login、Sidebar、Header、Mobile Navigation、Theme、Privacy、Loading/Error/Offline。

## Phase 3：BeeCount 财务源码导入

1. 固定 BeeCount Cloud upstream commit；
2. 复制实际需要的 UI / web-features / pages / contexts；
3. 建立 Finance Adapter；
4. 替换为 LifeTrace Session；
5. 放入 `FinanceWorkspace`；
6. 完成 Overview / Transactions / Accounts / Categories / Tags / Budgets / Ledgers / Calendar / Import；
7. 跑 CRUD / filter / chart / sync / import 测试；
8. 创建 `UPSTREAM.md` 并更新第三方声明。

## Phase 4：Dashboard

按 Shadcnblocks + Tremor + Catalyst 参考完全重做。

## Phase 5：计划系统

按 Shadcnblocks Todo / Calendar + Catalyst detail layout 重做 Execution、Calendar、Goals、Habits。

## Phase 6：健康与知识

Fitness / Health 重点参考 Tremor；Notes / English 重点参考 Catalyst / Preline；Review 参考 Shadcnblocks + Tremor。

## Phase 7：Search + Settings + System

Settings 按 Catalyst / Preline；Command Palette 按 shadcn；AI 入口可使用少量 Aceternity / Magic UI。

## Phase 8：Mobile / Tablet 专项

至少验证：360、390、430、768、1024 宽度。不能最后只补几个 media query。

## Phase 9：功能与设计双重对账

功能矩阵逐项标记：

```text
PASS
INTENTIONALLY_REMOVED
DESKTOP_ONLY
BLOCKED_BY_BACKEND
```

同时检查核心页面是否符合对应模板参考和统一 Design System。

## Phase 10：切换

更新根 `web:*` scripts、Docker/Caddy、CI 和生产构建入口，完成 smoke test。

## Phase 11：删除 legacy

切换稳定后删除：

```text
apps/desktop/web-client/
```

以及旧 browser CSS、route glue、旧 `BeeCountFinancePage.tsx`、`web-beecount.css` 和无引用 selector。

---

# 14. 测试门禁

必须通过：

```text
typecheck
unit tests
web build
production preview smoke test
```

核心 route 必测：URL 直达、refresh、back/forward、auth expiry、loading、empty、API error。

响应式至少：

```text
360 × 800
390 × 844
430 × 932
768 × 1024
1024 × 768
1366 × 768
1440 × 900
1920 × 1080
```

主题：Light / Dark / System。

财务专项至少覆盖：

```text
进入财务
切换账本
Overview
交易查询和多条件筛选
新增 / 编辑 / 删除交易
账户
分类
标签
预算
导入
Dark mode
收入/支出颜色方案
隐私金额
共享账本（启用时）
WebSocket 刷新（启用时）
```

财务行为对照 BeeCount Web，而不是旧 LifeTrace 财务页。

Accessibility 至少包括 keyboard、focus-visible、accessible name、form label、dialog focus trap、reduced motion。

---

# 15. Definition of Done

## Architecture

- [ ] Web 已迁移到独立 `apps/web`；
- [ ] 不依赖 legacy Web 运行；
- [ ] React Router 路由统一；
- [ ] Design System 只有一套；
- [ ] 不再新增 `.hx-*` / `.lt-*` 作为新 UI contract。

## Design

- [ ] App Shell 明确参考 Catalyst / shadcn / Preline；
- [ ] Dashboard 明确参考 Shadcnblocks / Tremor；
- [ ] Settings 明确参考 Catalyst / Preline；
- [ ] 数据页明确参考 Tremor；
- [ ] 每个核心页面有模板参考映射；
- [ ] Magic UI / Aceternity 仅用于局部增强；
- [ ] Desktop / Tablet / Mobile 完成；
- [ ] Light / Dark 完成。

## Finance

- [ ] 使用 BeeCount Cloud Web 源码级复用；
- [ ] 核心财务页面已迁移；
- [ ] LifeTrace Session 替代 BeeCount 独立登录；
- [ ] Finance Adapter 已建立；
- [ ] CRUD / Filter / Chart / Import 通过；
- [ ] BeeCount 作者和 License 保留；
- [ ] `UPSTREAM.md` 和第三方声明完成；
- [ ] 旧 `BeeCountFinancePage.tsx` 不再是正式入口。

## Quality

- [ ] typecheck / unit / build 通过；
- [ ] route smoke test 通过；
- [ ] responsive matrix 通过；
- [ ] theme matrix 通过；
- [ ] accessibility 基础门禁通过；
- [ ] 财务专项测试通过。

## Cleanup

- [ ] 生产已切到新 Web；
- [ ] legacy Web 删除；
- [ ] legacy browser CSS 删除；
- [ ] 无失效 route 和无引用 selector；
- [ ] 部署和文档已同步更新。

---

# 16. Agent 执行硬约束

1. 不要把任务解释成“继续优化旧 Web”。
2. 在新 `apps/web` 中重建。
3. **设计前必须先查看本文指定的模板参考，不能凭 Agent 默认审美自由发挥。**
4. 每个核心页面必须声明 Primary / Secondary Template Reference。
5. shadcn/ui 是组件规范核心；Catalyst 是 App Shell / Settings 的主要布局参考；Shadcnblocks / Preline 是完整页面参考；Tremor 是数据表达参考。
6. Magic UI / Aceternity 只做局部增强，禁止全站炫技。
7. TailAdmin 只在复杂高密度信息页参考，不把 LifeTrace 做成企业 Admin。
8. 财务必须研究 BeeCount Cloud Web 实际源码后移植，禁止凭截图仿写。
9. BeeCount 特有逻辑进入 `features/finance/beecount`，LifeTrace 特有对接进入 Adapter。
10. 每个 feature 必须同时完成 Desktop / Mobile / Dark / Empty / Loading / Error。
11. 每个 Phase 结束必须构建和测试。
12. legacy 只能在正式切换和回归全部通过之后删除。
13. BeeCount 源码复用必须保留作者、许可与来源记录。

---

# 17. 最终产品结构

```text
LifeTrace Personal OS
│
├── Global Design Language
│   ├── shadcn/ui component system
│   ├── Catalyst application layout
│   ├── Shadcnblocks page composition
│   ├── Preline complete-page references
│   ├── Tremor analytics patterns
│   └── Magic UI / Aceternity micro-interactions
│
├── LifeTrace Native Modules
│   ├── Today
│   ├── Execution
│   ├── Calendar
│   ├── Habits
│   ├── Fitness / Health
│   ├── Notes
│   ├── English
│   ├── Review
│   ├── Search
│   └── Settings
│
└── Finance Workspace
    └── BeeCount Web derived implementation
        ├── Overview
        ├── Transactions
        ├── Calendar
        ├── Ledgers
        ├── Budgets
        ├── Accounts
        ├── Categories
        ├── Tags
        └── Import
```

最终目标不是“做一个像 shadcn 的网站”或“把 BeeCount 塞进 LifeTrace”，而是：

> 使用最开始筛选的优秀前端模板体系建立统一、成熟的 LifeTrace Personal OS；对通用 UI 采用行业成熟模式，对财务领域直接复用 BeeCount 的成熟实现，避免重复造轮子。