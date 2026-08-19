# Web UI 完全重构实施方案

## 1. 文档目的

本文档定义 LifeTrace **Web 端完全推倒重构**的目标架构、视觉体系、页面规划、财务模块复用方式、实施顺序、测试门禁和最终切换标准，作为后续 Agent 执行 Web 重构时的权威依据。

本次不是在现有 `apps/desktop/web-client` 上继续做增量换肤，也不是继续叠加 `web-*.css`。目标是重新建立一个独立、长期可维护的 Web 应用，在功能验证完成后一次性切换入口，再删除旧 Web 实现。

核心产品定位：

> LifeTrace Web 是一个 Personal OS / Personal Dashboard，而不是传统企业 Admin 后台。

目标体验：

- 信息密度高但不拥挤；
- 首页围绕“今天、趋势、下一步行动”组织；
- 视觉克制，优先信息层级而不是装饰；
- 桌面浏览器、平板和手机浏览器均为一等平台；
- 业务模块在同一设计系统内保持一致；
- 财务栏目不重新设计，直接复用 BeeCount Cloud Web 的成熟财务工作区。

---

## 2. 已确认的架构决策

以下决策为本轮重构的硬约束。

### 2.1 Web 前端允许完全推倒

旧 Web 实现仅作为：

- 功能清单参考；
- API 调用参考；
- 业务行为回归参考；
- 数据契约参考。

旧组件、旧 CSS、旧页面结构**不是必须保留的实现资产**。

允许：

- 重写 App Shell；
- 重写路由；
- 重写页面组件；
- 重写 Design System；
- 重写响应式布局；
- 重写 Dashboard；
- 重组目录；
- 将 Web 从 `apps/desktop/web-client` 迁出，成为独立 `apps/web` 应用；
- 在最终切换后删除旧 Web UI 代码。

不允许因为“兼容旧 CSS”而继续扩大历史包袱。

### 2.2 后端与数据协议不是本轮推倒对象

本轮重点是前端重构。

默认继续复用：

- LifeTrace Cloud；
- 现有通用同步协议；
- 现有实体数据模型；
- 认证与会话能力；
- 已实现的 BeeCount 兼容接口；
- 现有业务数据。

如果新 UI 暴露出接口缺口，可以补 API，但不能为了迁就页面设计无理由重做后端。

### 2.3 财务栏目必须源码级复用 BeeCount Web

财务模块不再以 LifeTrace 当前 `BeeCountFinancePage.tsx` + `web-beecount.css` 为长期实现。

目标来源：

```text
TNT-Likely/BeeCount-Cloud
└── frontend/
    ├── apps/web/
    └── packages/
        ├── api-client/
        ├── ui/
        └── web-features/
```

当前调研基线来自 BeeCount Cloud `main` 分支；实施时必须记录实际复用的 upstream commit。

BeeCount 主仓库 README 已明确 BeeCount Cloud “自带 Web 管理端”，Web 与手机端同步，并包含 Web 首页、交易列表、设备/备份等能力。

财务功能应尽量直接复用 BeeCount Web 的：

- 页面结构；
- 财务交互；
- 数据筛选；
- 表格；
- 图表；
- 财务状态管理；
- 账本切换；
- 账户、分类、标签、预算等页面逻辑；
- 移动端财务导航与响应式行为；
- 共享账本相关交互（后端兼容时）；
- 导入流程（LifeTrace 后端能力满足时）。

不是“参考 BeeCount 风格重新写一个像 BeeCount 的页面”，而是**复用 BeeCount Web 源码并做适配**。

---

## 3. 当前代码基线

### 3.1 LifeTrace 当前 Web

当前 Web 位于：

```text
apps/desktop/web-client/
```

现状包括：

- React；
- Vite Browser 构建；
- 自定义路由；
- `AppShell.tsx`；
- `DashboardPage.tsx`；
- 多个业务 Page；
- `web-tokens.css`；
- `web-primitives.css`；
- `web-shell.css`；
- `web-workspaces.css`；
- `web-beecount.css`；
- 其他历史 CSS。

当前实现已经能用，但存在明显的演进成本：

1. Web 仍嵌套在 `apps/desktop` 下，架构语义不清；
2. 旧 `styles.css` 与多层 `web-*.css` 同时存在；
3. `.hx-*`、`.lt-*`、业务专属 class 长期并存；
4. UI Primitive 更多是 CSS 约定而不是正式 React 组件；
5. 多页面自行处理布局，响应式规则分散；
6. Web 与 Desktop 有部分代码引用关系，不利于独立演进；
7. 财务栏目目前是 LifeTrace 自定义只读/适配式展示，和 BeeCount Web 的完整财务体验存在重复开发。

因此旧 Web 不再作为新设计系统的地基。

### 3.2 BeeCount Cloud Web

BeeCount Cloud Web 当前技术栈包括：

```text
React
React Router
Vite
Tailwind CSS
shadcn 风格组件体系
class-variance-authority
clsx
tailwind-merge
cmdk
Lucide React
Recharts
Vitest
```

其 `components.json` 已按 shadcn schema 配置，使用 CSS Variables 和 neutral base color。

主要财务页面已经独立拆分，包括：

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

此外还存在：

- Profile / Appearance；
- Devices；
- AI；
- Shared Ledger；
- WebSocket 实时刷新；
- Context Provider；
- Page Data Cache；
- Attachment Cache；
- Lazy Route；
- Mobile Bottom Navigation。

这意味着财务模块没有必要再次从零实现。

---

## 4. 目标目录结构

重构完成后，Web 应成为与 Desktop 平级的一等应用。

推荐目标：

```text
apps/
├── desktop/
├── web/
│   ├── package.json
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── app/
│       │   ├── App.tsx
│       │   ├── router.tsx
│       │   ├── providers.tsx
│       │   └── bootstrap.ts
│       ├── layouts/
│       │   ├── AppShell.tsx
│       │   ├── AppSidebar.tsx
│       │   ├── AppHeader.tsx
│       │   ├── MobileNavigation.tsx
│       │   └── PageLayout.tsx
│       ├── components/
│       │   ├── ui/
│       │   ├── data-display/
│       │   ├── feedback/
│       │   └── navigation/
│       ├── features/
│       │   ├── dashboard/
│       │   ├── execution/
│       │   ├── habits/
│       │   ├── health/
│       │   ├── fitness/
│       │   ├── finance/
│       │   │   └── beecount/
│       │   ├── notes/
│       │   ├── english/
│       │   ├── review/
│       │   ├── search/
│       │   └── settings/
│       ├── services/
│       ├── stores/
│       ├── hooks/
│       ├── lib/
│       ├── types/
│       └── styles/
│           ├── globals.css
│           └── tokens.css
└── photo-challenge-pwa/

packages/
├── lifetrace-contracts/        # 如已有则继续使用现有契约位置
├── web-ui/                     # 需要跨 feature 复用时再抽取
└── ...
```

### 4.1 为什么迁出 `apps/desktop/web-client`

Web 与 Desktop 是两个不同的运行时：

- Web 有浏览器路由、PWA、响应式、Cookie/CORS 等约束；
- Desktop 有 Tauri IPC、本地文件、本地数据库、原生能力；
- Web 应能独立构建、部署、测试；
- Web 不应该为了复用少量组件依附在 Desktop 目录下。

因此完全重构时同时修正目录边界。

### 4.2 最终切换前允许双轨存在

重构期间允许：

```text
apps/desktop/web-client/   # legacy，仅用于回归对照
apps/web/                  # new，新的唯一开发目标
```

新 Web 达到切换门禁后：

1. 更新根目录 `web:*` scripts；
2. 更新部署配置；
3. 更新 Caddy/Docker 构建路径；
4. 切流量到 `apps/web`；
5. 删除旧 `apps/desktop/web-client`。

这仍属于“完全推倒重构”，只是使用并行重建降低线上回归风险，而不是在旧代码上渐进修补。

---

## 5. 新 Web 技术栈

### 5.1 基线

新应用建议使用：

```text
React 19
TypeScript
Vite
React Router
Tailwind CSS
shadcn/ui 模式组件
Lucide React
Recharts
Zustand（只用于真正需要的客户端状态）
Vitest / 现有测试体系
```

### 5.2 Tailwind 版本策略

BeeCount Cloud 当前 Web 使用 Tailwind 3.x。

为了最大化 BeeCount 财务源码复用，**不要在“复制 BeeCount + 重构 LifeTrace + Tailwind 4 升级”三个大变化中同时做升级**。

建议：

1. 新 Web 第一阶段固定一个与 BeeCount 源码兼容的 Tailwind 配置；
2. 完成财务和全站切换；
3. Tailwind 大版本升级作为独立后续任务。

这样可以避免大量 utility、插件和 config 差异混入本轮重构。

### 5.3 React Router

新 Web 不继续维护手写 History 路由系统。

统一采用正式路由树：

```text
/
/login
/app
/app/today
/app/execution
/app/calendar
/app/habits
/app/fitness
/app/finance/*
/app/notes
/app/english/*
/app/review
/app/search
/app/settings/*
```

所有一级模块必须支持 URL 直达和浏览器前进/后退。

页面级代码采用 lazy import，避免所有模块首屏一次加载。

---

## 6. 设计方向

LifeTrace 非财务页面采用：

```text
shadcn/ui 的组件语义
+ Catalyst 的 Application UI 信息架构
+ Tremor 的数据表达方式
+ Linear / Vercel 的克制层级
+ Apple Health 式健康数据组织
```

### 6.1 核心关键词

```text
Calm
Focused
Personal
Dense but readable
Data-aware
Consistent
Responsive
```

### 6.2 禁止项

- 传统 AdminLTE 风格；
- 满屏渐变；
- 无意义玻璃拟态；
- 所有内容都包 Card；
- 20px+ 圆角滥用；
- 每张卡都有大阴影；
- Emoji 当正式图标；
- Dashboard 十几个同优先级 KPI；
- 手机端直接把桌面三列压成一列；
- 每个 feature 自己发明一套按钮、Dialog、Toast。

---

## 7. Design System

## 7.1 Token

新 Web 从第一天就只保留一套设计 Token。

推荐 shadcn 语义：

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
```

额外业务语义：

```text
success
warning
info
income
expense
chart-1 ... chart-n
```

### 7.2 LifeTrace 品牌色

非财务区域继续使用低饱和绿色作为品牌强调。

- 页面主体以 neutral 为主；
- 品牌色用于 active、primary action、focus ring；
- 不通过大面积绿色背景建立品牌感。

### 7.3 财务颜色

BeeCount 财务模块保留 BeeCount 已有“收入/支出颜色方案”的业务能力。

不得为了强行统一 LifeTrace 主色而破坏：

- 收入/支出颜色切换；
- 分类图表颜色；
- BeeCount 财务语义。

### 7.4 Typography

建议：

```text
Page title      24–28 / 600
Section title   18–20 / 600
Card title      14–16 / 600
Body            14 / 400
Secondary       12–13 / 400
Metric          24–32 / 600
Micro           11–12 / 500
```

### 7.5 Radius

```text
6px   micro control
8px   button / input
10px  regular card
12px  dialog / large panel
999px pill / badge / avatar
```

BeeCount 已有组件中合理的圆角不需要为“统一数字”做无价值重写。

### 7.6 Shadow

普通信息层：`surface + border`。

阴影主要用于：

- Popover；
- Dropdown；
- Dialog；
- Floating toolbar；
- 明确可浮起的交互元素。

---

## 8. 全局组件体系

新 Web 建立真正的 React UI 层，不再以全局 `.hx-*` / `.lt-*` 选择器作为主要抽象。

### 8.1 基础组件

```text
components/ui/
├── button.tsx
├── input.tsx
├── textarea.tsx
├── select.tsx
├── checkbox.tsx
├── switch.tsx
├── badge.tsx
├── card.tsx
├── tabs.tsx
├── table.tsx
├── dialog.tsx
├── alert-dialog.tsx
├── drawer.tsx
├── sheet.tsx
├── dropdown-menu.tsx
├── popover.tsx
├── tooltip.tsx
├── command.tsx
├── skeleton.tsx
├── separator.tsx
├── scroll-area.tsx
├── progress.tsx
└── toast.tsx
```

### 8.2 应用组件

```text
components/navigation/
├── AppSidebar.tsx
├── SidebarGroup.tsx
├── MobileNavigation.tsx
└── CommandPalette.tsx

components/data-display/
├── MetricCard.tsx
├── TrendCard.tsx
├── ChartCard.tsx
├── Timeline.tsx
├── ActivityList.tsx
├── DataTable.tsx
└── EmptyState.tsx

components/feedback/
├── AppLoading.tsx
├── RouteLoading.tsx
├── ErrorState.tsx
├── OfflineState.tsx
└── ConflictNotice.tsx
```

### 8.3 页面禁止重复实现

禁止页面自行再实现：

- Button；
- Input；
- Dialog；
- Dropdown；
- Toast；
- Empty State；
- Skeleton；
- Tooltip；
- Table 基础样式；
- Page Header；
- KPI Card。

---

## 9. App Shell 完全重写

新 App Shell 不从旧 `.hx-shell` 继续修改。

### 9.1 Desktop

```text
┌──────────────────────────────────────────────────────────────┐
│ Sidebar │ Page header                         Search  Actions │
│         ├────────────────────────────────────────────────────┤
│ Today   │                                                    │
│ Plan    │                 Page Content                       │
│ Habit   │                                                    │
│ Health  │                                                    │
│ Finance │                                                    │
│ Notes   │                                                    │
│ ...     │                                                    │
│         │                                                    │
│ User    │                                                    │
└──────────────────────────────────────────────────────────────┘
```

原则：

- Sidebar 220–248px；
- 支持 collapsed；
- 一级导航按语义分组；
- Top Header 比当前更薄；
- 页面标题和页面 action 属于内容层，不使用巨大 Hero；
- 全局搜索支持快捷键；
- 在线/同步状态只在必要时显式显示。

### 9.2 Mobile

手机端独立信息架构：

```text
Top bar
  ↓
Page
  ↓
Primary bottom nav
  ↓
More / Sheet
```

不保留桌面 Sidebar 的缩小版。

### 9.3 推荐一级导航

```text
TODAY
  今日

PLAN
  计划与待办
  日历
  坚持

HEALTH
  健身
  健康

FINANCE
  财务

KNOWLEDGE
  笔记
  英语
  复盘

SYSTEM
  搜索
  设置
```

---

## 10. Dashboard 完全重做

首页不是模块目录，也不是全业务 KPI 墙。

### 10.1 首页必须回答

用户打开 LifeTrace 时，5 秒内应知道：

1. 今天最重要的事情是什么；
2. 今天完成多少；
3. 哪件事应该下一步处理；
4. 最近整体状态有没有异常；
5. 是否需要进入某个业务模块。

### 10.2 推荐结构

```text
Greeting + Date

Today Focus
┌──────────────────────────────────────────────────┐
│ 今日主要目标 / 完成度 / 下一步                  │
└──────────────────────────────────────────────────┘

Today
┌─────────────────────────┐  ┌─────────────────────┐
│ Tasks / Habits          │  │ Schedule            │
└─────────────────────────┘  └─────────────────────┘

Trends
┌──────────────────────────────────────────────────┐
│ 训练 / 学习 / 支出 / 习惯的少量关键趋势         │
└──────────────────────────────────────────────────┘

Recent Activity        Quick Actions
```

### 10.3 KPI 上限

首屏最多 4 个等权指标。

更多指标进入对应业务页。

---

# 11. 财务模块：BeeCount Web 源码级复用

这是本方案最重要的专项约束。

## 11.1 正确的上游来源

BeeCount 财务 Web 源码来自：

```text
https://github.com/TNT-Likely/BeeCount-Cloud
```

不是 BeeCount 官网 `BeeCount-Website`。

BeeCount 主客户端仓库中所谓 “Web (Self-Hosted)” 指向 BeeCount Cloud；BeeCount Cloud 的前端代码位于：

```text
frontend/apps/web/
frontend/packages/ui/
frontend/packages/web-features/
frontend/packages/api-client/
```

## 11.2 复用原则

优先级：

```text
直接复用源代码
    > 抽取适配
    > 小范围视觉调整
    > 重新实现
```

只有在 BeeCount 代码和 LifeTrace 架构确实冲突时才重写。

禁止：

> 看着 BeeCount 截图重新写一份“差不多”的财务页面。

## 11.3 必须复用的财务页面

至少包含：

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

根据 LifeTrace BeeCount 兼容后端实际能力继续复用：

- Shared Ledger dialogs；
- Transaction edit dialogs；
- Entity dialogs；
- Category icon；
- Attachment cache；
- 实时同步刷新；
- AI Parse Transaction；
- Profile 中的财务配色偏好。

## 11.4 不直接复制 BeeCount 全局壳

以下 BeeCount Web 能力不应直接取代 LifeTrace 全局系统：

```text
BeeCount LoginPage
BeeCount 全局 AppShell
BeeCount 全局 AppHeader
BeeCount Admin Users
BeeCount Admin Backup
BeeCount Admin Cleanup
```

原因：

- LifeTrace 有统一账号体系；
- LifeTrace 有自己的全局 Sidebar；
- LifeTrace 有自己的系统设置和云服务管理；
- 财务只是 LifeTrace 的一个模块。

因此应采用：

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

## 11.5 财务路由

建议：

```text
/app/finance
/app/finance/transactions
/app/finance/calendar
/app/finance/ledgers
/app/finance/budgets
/app/finance/accounts
/app/finance/categories
/app/finance/tags
/app/finance/import
```

`/app/finance` 对应 BeeCount `OverviewPage`。

桌面端财务工作区可在内容区顶部提供二级 Tab / Subnav；手机端使用财务模块内部 Sheet 或横向 tabs，不额外创建第二套底部主导航。

## 11.6 API 适配边界

BeeCount 页面不要直接到处改成 LifeTrace API 调用。

建立统一 Finance Adapter：

```text
features/finance/
├── beecount/          # 上游源码改造区
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

BeeCount UI 继续按 BeeCount 的领域模型工作，Adapter 负责连接 LifeTrace 已实现的 BeeCount 兼容接口。

目标：以后 BeeCount upstream 更新时，页面 diff 尽量小。

## 11.7 Context 复用

BeeCount Web 已经把很多跨页面状态拆成 Provider。

应优先保留并适配：

- LedgersContext；
- AttachmentCacheContext；
- SharedLedgerResourcesContext；
- PageDataCacheContext；
- SyncSocketContext。

不要把这些能力重新塞进一个巨大的 LifeTrace Zustand store。

## 11.8 图表复用

BeeCount 使用 Recharts。

LifeTrace 新 Web 也选择 Recharts，因此：

- 财务图表直接保留；
- 非财务图表也以相同 chart primitive 为基础；
- 全站只维护一套 Recharts 封装；
- Tooltip / Legend / ResponsiveContainer 行为统一。

## 11.9 财务视觉

财务模块优先保持 BeeCount Web 自身成熟视觉，不强行把所有内容改成 LifeTrace 首页的视觉。

允许统一：

- 外层 Page padding；
- LifeTrace 顶级 header；
- 字体 fallback；
- focus ring；
- 无障碍规则；
- 全局 dark/light 切换入口。

应保留：

- BeeCount 财务信息密度；
- 财务 card/table 结构；
- 收入/支出颜色逻辑；
- 分类图表；
- 账本 selector；
- 交易筛选逻辑；
- 财务二级导航。

## 11.10 上游追踪

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
Files intentionally not imported
Sync procedure
License notice
```

每次同步 BeeCount 新版本时必须能回答：

> “哪些代码来自 BeeCount，哪些是 LifeTrace 自己改的？”

## 11.11 License 和作者信息

BeeCount Cloud 当前许可允许个人/非商业使用、修改与分发，但要求：

- 保留版权声明；
- 保留许可协议；
- 不移除作者信息；
- 修改版本公开源代码；
- 商业集成需要额外商业许可。

因此 LifeTrace 复用 BeeCount Web 时必须：

1. 保留 BeeCount 原始作者和许可；
2. 在仓库 `THIRD-PARTY-NOTICES` 或对应第三方声明中增加 BeeCount Cloud；
3. 在复用目录保留 `UPSTREAM.md`；
4. 不能把 BeeCount 代码改名后声称为 LifeTrace 原创；
5. 如果未来 LifeTrace 商业化，在商业化前重新审查 BeeCount 许可并获取必要授权。

---

## 12. 各模块页面重构要求

### 12.1 计划与待办

结构：

```text
Inbox
Today
Upcoming
Projects
Completed
```

桌面：列表 + detail drawer。

手机：列表 → 全屏 detail。

高频新建操作必须 <= 2 次点击。

### 12.2 日历

至少支持：

- 月；
- 周；
- 日 / Agenda；
- 时间块；
- 任务和事件统一显示；
- 今日快速跳转。

手机默认 Agenda / Day，不强行展示完整桌面月历。

### 12.3 坚持 / Habits

核心是：

- 今日完成；
- streak；
- 近 7/30 天趋势；
- Heatmap；
- 快速打卡。

### 12.4 健身

参考 Apple Health / modern fitness dashboard：

```text
本周训练
训练量趋势
最近训练
动作 / 肌群统计
身体指标（有数据才显示）
```

避免“后台表格 + 大量表单”的表现方式。

### 12.5 笔记

桌面采用：

```text
Notes List | Editor
```

可扩展：

```text
Folders | Notes | Editor
```

手机采用单页层级导航。

### 12.6 英语

至少保留：

- 阅读列表；
- 阅读器；
- 高亮；
- 快捷笔记；
- 阅读完成；
- 学习历史；
- 生词 / 短语入口。

阅读器必须保持内容优先，减少 App Shell 干扰。

### 12.7 复盘

首页展示：

- 今天；
- 近期 streak；
- 最近 7 天摘要。

编辑区比普通 Dashboard 更偏内容编辑器。

### 12.8 设置

采用 Catalyst / shadcn 常见 Settings Layout：

```text
Profile
Appearance
Cloud & Sync
Devices
Privacy
Security
Data
About
```

危险操作独立 Danger Zone。

---

## 13. 全局搜索 / Command Palette

新 Web 把搜索升级成一等能力。

入口：

- Header 搜索；
- `Ctrl/Cmd + K`。

搜索范围：

```text
任务
项目
坚持
训练
财务交易
账本
笔记
英语记录
复盘
设置入口
```

支持直接命令：

```text
新建任务
记录支出
开始训练
新建笔记
打开今日
打开财务
```

财务搜索可复用 BeeCount transaction/filter 能力，但要通过统一 Search Adapter 暴露。

---

## 14. 响应式规范

统一断点建议：

```text
< 640px       Mobile
640–1023px    Tablet
>= 1024px     Desktop
>= 1440px     Wide
```

### Mobile

- 无桌面 Sidebar；
- 主导航 bottom nav；
- 次级功能进入 More Sheet；
- 表格转 List/Card；
- Dialog 优先 Bottom Sheet / Fullscreen；
- 44px 最小主要点击区域；
- sticky action 不遮挡安全区。

### Tablet

- Sidebar 可 collapsed；
- 双列布局有限使用；
- detail 可 Drawer。

### Desktop

- 稳定 Sidebar；
- 支持双/三栏工作区；
- 更高信息密度；
- table 不强制转 card。

---

## 15. 状态管理边界

不要因为重构 UI 顺便建立一个“全局超级 Store”。

### Server / Cloud State

优先放在：

- API client；
- page loader / feature hook；
- feature context；
- cache layer。

### Client UI State

Zustand 只存：

- sidebar collapsed；
- theme；
- privacy mode；
- command palette；
- 少量跨 route UI 状态。

### URL State

以下优先放 URL：

- 搜索词；
- Filter；
- 排序；
- 时间范围；
- 当前财务子页面；
- 当前账本（可结合用户偏好）；
- 分页。

这样刷新页面和分享链接不会丢上下文。

---

## 16. 主题与 Dark Mode

新系统从底层支持：

```text
system
light
dark
```

要求：

- 所有颜色来自 token；
- 图表 dark mode 可读；
- BeeCount finance dark mode 保留；
- 不允许页面用硬编码白色背景绕过 theme；
- 浏览器 `theme-color` 与当前主题同步。

---

## 17. 动效

Magic UI / Aceternity 只作为局部参考。

允许：

- route fade/slide；
- number ticker；
- progress；
- command palette；
- dialog / drawer transition；
- 成就完成反馈。

禁止：

- 常驻发光；
- 无限浮动；
- 粒子背景；
- 每张 card stagger animation；
- 对业务操作造成延迟的动画。

必须支持 `prefers-reduced-motion`。

---

# 18. 实施阶段

## Phase 0：功能盘点与冻结

先建立旧 Web 功能清单。

逐页记录：

```text
Route
Feature
Read API
Write API
Permission
Desktop behavior
Mobile behavior
Edge states
Test coverage
Replacement page
```

同时列出：

- 旧页面；
- 旧公共组件；
- 旧 CSS；
- Desktop-only 能力；
- Web-only 能力。

没有盘点完成，不开始删除旧代码。

## Phase 1：建立 `apps/web`

完成：

- 独立 package；
- Vite；
- TypeScript；
- React Router；
- Tailwind；
- shadcn-compatible UI；
- tokens；
- lint/test/build；
- 环境变量；
- API base；
- Error Boundary；
- telemetry/observability。

此阶段页面只需要空壳。

## Phase 2：App Shell + Auth

重写：

- Login；
- Session bootstrap；
- Sidebar；
- Header；
- Mobile nav；
- Theme；
- Privacy；
- Loading / Error / Offline。

完成后所有一级 Route 能进入空页面。

## Phase 3：BeeCount 财务源码导入

这是第一批正式业务页面。

步骤：

1. 固定 BeeCount Cloud upstream commit；
2. 复制 `ui` 中实际需要的组件；
3. 复制 `web-features` 中财务实际需要的逻辑；
4. 复制财务 pages / dialogs / hooks / contexts；
5. 建立 `finance/adapters`；
6. 接 LifeTrace BeeCount-compatible backend；
7. 将 BeeCount Auth 替换为 LifeTrace Session；
8. 将 BeeCount AppShell 替换为 LifeTrace FinanceWorkspace；
9. 保留 BeeCount 财务二级导航；
10. 跑完整财务 CRUD / filter / chart / import / sync 测试；
11. 新增 `UPSTREAM.md` 和第三方声明。

阶段完成标准：

> LifeTrace `/app/finance/*` 的核心体验与 BeeCount Web 对应功能等价，而不是继续使用旧的 `BeeCountFinancePage.tsx`。

## Phase 4：Dashboard

基于新 Design System 重写首页。

接入真实：

- Task；
- Habit；
- Workout；
- Finance summary；
- Notes / English / Review activity。

财务 summary 只做聚合入口，不在首页重复实现 BeeCount 财务中心。

## Phase 5：计划系统

重写：

- Execution；
- Calendar；
- Goals；
- Habits。

## Phase 6：健康与知识

重写：

- Fitness；
- Health；
- Notes；
- English；
- Review。

## Phase 7：Search + Settings + System

完成：

- Command Palette；
- Global Search；
- Profile；
- Appearance；
- Devices；
- Cloud；
- Privacy / Security；
- Data lifecycle UI。

## Phase 8：Mobile / Tablet 专项

逐页在真实宽度验证，而不是最后只补几个 media query。

重点：

- 360px；
- 390px；
- 430px；
- 768px；
- 1024px。

## Phase 9：功能对账

按 Phase 0 的功能矩阵逐项对比旧 Web。

每一项必须是：

```text
PASS
INTENTIONALLY_REMOVED
DESKTOP_ONLY
BLOCKED_BY_BACKEND
```

不能有“不确定是否迁移”。

## Phase 10：切换

完成：

- 根 `web:dev` 指向 `apps/web`；
- 根 `web:build` 指向 `apps/web`；
- `browser:*` 兼容命令按需要保留或迁移；
- Docker/Caddy 改为新 dist；
- CI 改为新 Web；
- 部署验证；
- 生产 smoke test。

## Phase 11：删除 legacy Web

确认切换稳定后删除：

```text
apps/desktop/web-client/
```

以及只为旧 Web 存在的：

- Vite browser config；
- legacy CSS；
- legacy route glue；
- legacy browser-specific adapter；
- 无引用 class；
- 旧 BeeCountFinancePage；
- `web-beecount.css`。

如果某段代码仍被 Desktop 使用，则迁移到正确共享目录后再删除旧目录。

---

## 19. 测试门禁

完全重构不能靠截图判断完成。

### 19.1 Build

必须通过：

```text
typecheck
unit tests
web build
production preview smoke test
```

### 19.2 Route

每个正式 route 验证：

- 直接输入 URL；
- refresh；
- browser back；
- browser forward；
- auth expiry；
- loading；
- API error；
- empty state。

### 19.3 Responsive

至少：

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

### 19.4 Theme

每个核心模块必须检查：

- Light；
- Dark；
- System；
- reload 后保持。

### 19.5 财务专项

至少覆盖：

```text
登录后进入财务
切换账本
Overview
交易查询
多条件筛选
新增交易
编辑交易
删除交易
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

财务测试要对照 BeeCount Web 行为，而不是只对照旧 LifeTrace 财务页。

### 19.6 Accessibility

至少满足：

- keyboard navigation；
- focus visible；
- button accessible name；
- form label；
- dialog focus trap；
- 颜色不是唯一状态载体；
- reduced motion。

---

## 20. 性能门禁

目标：

- Route lazy load；
- 财务页不进入首页首包；
- Rich editor 不进入非笔记首包；
- 大图表按页加载；
- 大列表支持 pagination / virtualization；
- 不进行无效全局 re-render；
- 不因 Context 滥用导致整个应用频繁刷新。

BeeCount Web 已采用 section lazy loading，这一策略应保留。

---

## 21. Definition of Done

只有同时满足以下条件，才算“Web 完全重构完成”。

### Architecture

- [ ] 新 Web 已迁移到独立 `apps/web`；
- [ ] 不再依赖 `apps/desktop/web-client` 运行；
- [ ] 路由体系统一；
- [ ] Design System 只有一套；
- [ ] 没有继续新增 `.hx-*` / legacy style contract。

### UX

- [ ] 全新 App Shell；
- [ ] 全新 Dashboard；
- [ ] Desktop / Tablet / Mobile 均完成；
- [ ] Light / Dark 完成；
- [ ] Loading / Empty / Error / Offline 完成。

### Finance

- [ ] 财务中心使用 BeeCount Cloud Web 源码级复用方案；
- [ ] `OverviewPage` 等核心财务页面已迁移；
- [ ] LifeTrace Session 已替代 BeeCount 独立登录；
- [ ] Finance Adapter 已建立；
- [ ] 财务 CRUD 和筛选通过；
- [ ] BeeCount 作者与 License 信息保留；
- [ ] `UPSTREAM.md` 已创建；
- [ ] 第三方声明已更新；
- [ ] 旧 `BeeCountFinancePage.tsx` 不再作为正式入口。

### Quality

- [ ] typecheck 通过；
- [ ] unit tests 通过；
- [ ] web build 通过；
- [ ] 核心 route smoke test 通过；
- [ ] responsive matrix 通过；
- [ ] theme matrix 通过；
- [ ] 财务专项测试通过。

### Cleanup

- [ ] 已切换生产构建入口；
- [ ] 旧 Web 目录已删除；
- [ ] 旧 browser CSS 已删除；
- [ ] 无失效 route；
- [ ] 无无引用 legacy selector；
- [ ] 文档与部署脚本已更新。

---

## 22. Agent 执行约束

后续 Agent 执行本文档时必须遵守：

1. **不要把本任务解释成“继续优化旧 Web”。**
2. 在新 `apps/web` 中重建，不以兼容 legacy CSS 为设计目标。
3. 未完成新 Web 切换前，不删除旧 Web，旧 Web 只作为功能回归基线。
4. 财务模块先研究 BeeCount Cloud Web 实际源码再移植，禁止凭截图仿写。
5. BeeCount 财务代码尽量保持 upstream 结构，LifeTrace 特有逻辑进入 Adapter。
6. 每完成一个 feature 必须同时完成 desktop/mobile/dark/empty/loading/error 状态。
7. 每个 Phase 结束必须构建和测试，不能等最终一起修。
8. 删除 legacy 必须发生在正式切换和回归全部通过之后。
9. 不修改与本轮 UI 重构无关的后端业务语义。
10. 任何 BeeCount 源码复用都必须保留作者、许可和来源记录。

---

## 23. 最终目标

完成后，LifeTrace Web 应形成清晰的两层产品结构：

```text
LifeTrace Personal OS
│
├── LifeTrace Native Modules
│   ├── Today
│   ├── Execution
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

LifeTrace 负责统一账号、全局导航、个人管理体验和跨模块聚合；BeeCount 负责成熟、专业、完整的财务工作区。

这样既避免重复造一个质量更低的财务前端，又让 LifeTrace 的其他模块可以按照统一 Personal OS 设计语言彻底重构。