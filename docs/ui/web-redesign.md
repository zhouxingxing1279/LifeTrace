# Web UI 重构实施方案

## 1. 文档目的

本文档定义 LifeTrace **Web 端**的视觉、交互与前端组件体系重构方案，作为后续页面改造、组件迁移、验收和 Agent 执行时的权威实施依据。

本次重构目标不是把 LifeTrace 做成传统企业后台，而是形成一个长期可维护的 **Personal OS / Personal Dashboard**：信息密度适中、层级清晰、视觉克制、数据友好，并同时覆盖桌面浏览器、平板与手机浏览器。

参考设计语言：

- **shadcn/ui**：组件结构、语义化 Token、表单/弹窗/菜单/卡片等基础交互规范。
- **Tailwind Plus / Catalyst**：应用壳层、Sidebar、页面标题区、列表、设置页、数据表等成熟 Application UI 布局。
- **Tremor**：指标卡、趋势图、分析视图的数据表达方式。
- **Magic UI / Aceternity**：只借鉴少量局部动效与强调效果，不作为全站基础风格。

官方参考：

- https://ui.shadcn.com/
- https://ui.shadcn.com/docs/theming
- https://ui.shadcn.com/docs/installation/vite
- https://ui.shadcn.com/charts
- https://tailwindcss.com/plus/ui-kit
- https://tailwindcss.com/plus/ui-blocks/application-ui
- https://www.tremor.so/
- https://magicui.design/docs

> 注意：Catalyst / Tailwind Plus 属于商业资源。LifeTrace 只参考其布局、信息层级与交互模式，不复制未授权付费源码。

---

## 2. 重构范围

### 2.1 本次主要范围

仅以 Web 客户端为主：

```text
apps/desktop/web-client/
├── src/
│   ├── App.tsx
│   ├── AuthScreen.tsx
│   ├── components/
│   ├── pages/
│   ├── styles.css
│   ├── cloud-pages.css
│   ├── web-tokens.css
│   ├── web-primitives.css
│   ├── web-shell.css
│   ├── web-auth.css
│   ├── web-workspaces.css
│   ├── web-beecount.css
│   ├── web-features.css
│   └── web-photo-challenge.css
└── index.html
```

重构覆盖：

- Web App Shell；
- 导航体系；
- 首页 Dashboard；
- 任务 / 日历 / 目标 / 习惯；
- 健身与健康数据；
- 财务；
- 笔记 / 英语 / 复盘；
- 搜索；
- 设置 / 设备 / 云端状态；
- 登录注册；
- 移动端 Web 布局；
- 空状态、加载、错误、离线、冲突等系统状态。

### 2.2 非目标

本轮不以以下内容为主要目标：

- 不重构 Rust / Cloud 后端协议；
- 不修改现有数据模型和同步协议，除非 UI 确实暴露出契约缺失；
- 不要求 Tauri 桌面端与 Web 端逐像素一致；
- 不重做业务逻辑；
- 不为了换 UI 框架而删除稳定功能；
- 不在全站堆叠粒子、发光、渐变边框等营销页动效。

---

## 3. 当前 Web 端基线

当前 Web 客户端已经具备可继续演进的基础，不应推倒重写。

### 3.1 已存在的可复用能力

- `App.tsx` 已完成会话、在线状态、隐私金额、刷新、云端状态加载等顶层逻辑；
- `components/AppShell.tsx` 已经具备 Sidebar、Topbar、移动端导航和云端状态入口；
- `web-tokens.css` 已经定义 Web 专用颜色、字体、间距、圆角、阴影、尺寸、深色模式；
- `web-primitives.css` 已经定义按钮、状态、Panel、Metric 等基础样式；
- 页面已经按 `pages/` 拆分；
- Lucide React 已经作为统一图标体系；
- 当前构建仍然是 React + Vite，Web 构建入口独立于 Tauri 壳层。

### 3.2 当前主要问题

1. **样式层过多**：`styles.css` 与多份 `web-*.css` 并存，旧类名和新设计系统同时存在。
2. **组件语义不足**：大量页面仍直接依赖 `.hx-*`、`.lt-*`、页面专属 CSS，而不是稳定 React UI Primitive。
3. **视觉语言不完全统一**：旧 Web 风格、桌面端 Apple polish、当前 Web token 系统存在历史叠加。
4. **页面密度不一致**：部分页面接近传统后台，部分页面更像内容站，缺少统一的 Personal OS 视觉节奏。
5. **Dashboard 可继续收敛**：已有信息聚合能力，但应进一步降低“模块堆叠感”，强化今天、趋势、下一步行动三个层级。
6. **数据可视化体系不足**：数据页需要统一 Chart Card、Tooltip、Legend、Empty State 与时间范围选择。
7. **响应式规则需要统一**：不应让每个业务页面自己定义一套手机端行为。

因此，本次采取 **渐进式重构**：保留业务组件和路由，逐步替换视觉与组件层。

---

## 4. 目标设计定位

### 4.1 核心关键词

```text
Calm
Focused
Personal
Data-aware
Compact
Readable
Consistent
```

视觉上接近：

```text
shadcn/ui 的克制
+ Catalyst 的成熟应用布局
+ Tremor 的数据表达
+ Linear / Vercel 的信息层级
+ 少量 Apple Health 式健康数据卡片
```

### 4.2 必须避免

- 大面积高饱和渐变；
- 20px 以上圆角到处使用；
- 每张卡片都有阴影；
- 无意义 Glassmorphism；
- 过大的营销型 Hero；
- 每个模块使用独立主色；
- 所有内容都装进 Card；
- Dashboard 出现十几个同权重 KPI；
- 移动端单纯把桌面布局纵向压缩；
- 使用 Emoji 代替正式图标。

---

## 5. 技术策略

## 5.1 总原则

采用“**设计系统先行，页面渐进迁移**”的方案。

优先级：

```text
业务稳定性 > 组件一致性 > 页面视觉 > 动效
```

### 5.2 shadcn/ui 的使用方式

shadcn/ui 作为实际组件体系的主要参考，并允许在 Web 客户端逐步引入。

建议引入范围：

- Button
- Input / Textarea
- Select
- Checkbox / Switch
- Dialog / AlertDialog
- DropdownMenu
- Tooltip
- Tabs
- Badge
- Card
- Table
- Popover
- Command
- Sheet / Drawer
- Skeleton
- Separator
- ScrollArea

不要求第一阶段一次性迁移所有页面。

### 5.3 Tailwind 是否引入

建议仅在 **Web 客户端**引入 Tailwind CSS v4，并通过 Vite 插件接入，不强制桌面端同时迁移。

目标不是把全部旧 CSS 机械改写成 utility class，而是：

1. 新组件优先使用 Tailwind；
2. 设计 Token 继续作为唯一视觉语义来源；
3. 旧业务 CSS 在迁移期继续工作；
4. 页面迁移完成后逐批删除 legacy selector。

`vite.browser.config.ts` 只负责 Web，因此 Tailwind 插件可以只加入 browser 构建链路。

### 5.4 数据图表

实际实现优先使用 **Recharts + shadcn chart wrapper**。

理由：

- shadcn Charts 本身以 Recharts 为底层；
- Tremor 的图表设计语言可作为视觉参考；
- 不同时保留两套高度重叠的数据可视化组件系统；
- 后续可统一 Tooltip、Legend、颜色 Token 和响应式行为。

Tremor 用于参考以下设计：

- KPI + trend；
- Area / Line 趋势；
- Bar 对比；
- Donut 分类占比；
- 轻量时间区间切换。

### 5.5 Magic UI / Aceternity

不作为依赖基线。

只允许在以下位置使用少量效果：

- 首页首次加载的轻量 Blur/Fade；
- 数字变化 Number Ticker；
- 成就完成反馈；
- AI 助手入口；
- 年度/月度总结页面。

所有效果必须满足：

- `prefers-reduced-motion` 可关闭；
- 不影响内容阅读；
- 不引入持续高频动画；
- 不作为业务状态表达的唯一方式。

---

## 6. Web Design System

现有 `web-tokens.css` 继续作为 Web 端视觉契约，但需要收敛为更接近 shadcn 的语义层。

### 6.1 Token 分层

建议分为三层：

```text
Primitive Token
    ↓
Semantic Token
    ↓
Component Token
```

例如：

```text
neutral-50
    ↓
background / surface / muted
    ↓
card-background / sidebar-background
```

### 6.2 语义 Token

最终至少统一以下语义：

```text
background
foreground
surface
surface-raised
muted
muted-foreground
border
input
ring
primary
primary-foreground
secondary
secondary-foreground
success
warning
destructive
info
```

现有 `--lt-color-*` 可继续存在，第一阶段通过 alias 与 shadcn 风格语义 Token 对接，不要求一次性改名。

### 6.3 色彩

LifeTrace 保留低饱和绿色作为品牌强调色。

原则：

- 90% 页面使用中性色；
- 主绿色只用于选中、主按钮、关键趋势和轻量强调；
- 红色仅表示危险、失败、支出等必要语义；
- 黄色仅用于警告和等待状态；
- 财务、健康、英语等模块不再拥有互相冲突的整页主题色。

### 6.4 圆角

建议统一：

```text
6px   小标签 / 小控件
8px   Button / Input
10px  普通 Card
12px  Dialog / 大型容器
999px Badge / Avatar / Pill
```

Hero、Dashboard 主容器最多可使用 14–16px，但不作为全局默认。

### 6.5 阴影

默认 Card 主要依赖 `border + surface` 建立层级。

阴影只用于：

- Dropdown；
- Dialog；
- Floating panel；
- Hover 后确实需要抬升的可交互对象。

禁止所有普通信息卡使用明显大阴影。

### 6.6 字体层级

推荐：

```text
Page title      24–28 / 600
Section title   18–20 / 600
Card title      14–16 / 600
Body            14 / 400
Secondary       12–13 / 400
Metric          24–32 / 600
Micro label     11–12 / 500
```

中文界面避免过多全大写英文 Eyebrow。只在 ANALYTICS、TODAY 等少量视觉标签中保留。

---

## 7. 组件体系重构

建议在 Web 端建立：

```text
web-client/src/components/ui/
├── button.tsx
├── input.tsx
├── textarea.tsx
├── card.tsx
├── badge.tsx
├── dialog.tsx
├── dropdown-menu.tsx
├── sheet.tsx
├── tabs.tsx
├── tooltip.tsx
├── table.tsx
├── skeleton.tsx
├── empty-state.tsx
├── metric.tsx
├── chart.tsx
└── index.ts
```

再建立业务级布局组件：

```text
web-client/src/components/layout/
├── AppSidebar.tsx
├── AppHeader.tsx
├── MobileNavigation.tsx
├── PageHeader.tsx
├── PageContainer.tsx
├── ContentGrid.tsx
└── DetailDrawer.tsx
```

以及数据展示组件：

```text
web-client/src/components/data-display/
├── MetricCard.tsx
├── TrendCard.tsx
├── ActivityList.tsx
├── Timeline.tsx
├── DataTable.tsx
├── ChartCard.tsx
└── ProgressRing.tsx
```

### 7.1 组件约束

页面禁止重复实现：

- Button；
- Badge；
- Modal / Dialog；
- Dropdown；
- Input；
- Empty State；
- Loading；
- Toast；
- Tooltip；
- Page Header；
- Metric Card。

发现重复后应提升为公共组件，而不是继续创建 `.xxx-button`、`.xxx-card`。

---

## 8. App Shell 重构

当前 `AppShell.tsx` 保留职责，但拆分结构。

目标：

```text
┌──────────────────────────────────────────────┐
│ Sidebar │  Page Header / Global Actions      │
│         ├────────────────────────────────────┤
│         │                                    │
│         │  Page Content                      │
│         │                                    │
└──────────────────────────────────────────────┘
```

### 8.1 Sidebar

桌面宽屏：

- 默认宽度约 240px；
- 可折叠为 64–72px；
- 折叠时只保留图标并提供 Tooltip；
- 分组标签降低视觉权重；
- 当前页面使用柔和背景 + 主色文字，而不是高对比整块背景；
- 底部固定账户、同步状态、设置。

建议导航分组：

```text
概览
  今日

行动
  计划与待办
  日历
  习惯

健康
  健身

知识
  笔记
  英语
  复盘

财务
  财务中心

系统
  搜索
  AI 管家
  设备
  设置
```

最终分组以当前真实 Route 为准，不为了视觉设计删除已有入口。

### 8.2 顶部区域

Topbar 改为更轻量的 Page Header：

左侧：

- 页面标题；
- 一行简短说明，可按页面隐藏。

右侧：

- Search；
- Sync；
- Privacy；
- 页面级 Primary Action。

日期不再在所有页面作为高权重固定元素；只在 Dashboard、Calendar 等与日期强相关页面使用。

### 8.3 全局搜索

桌面端支持：

```text
Ctrl/Cmd + K
```

打开 Command Palette，搜索：

- 页面；
- 任务；
- 笔记；
- 计划；
- 财务记录；
- 英语记录。

移动端从 Header Search 图标进入全屏 Sheet。

---

## 9. 首页 Dashboard 重构

Dashboard 是本轮优先级最高的页面。

目标不是展示所有数据，而是回答三个问题：

1. **我今天最应该做什么？**
2. **我目前状态怎么样？**
3. **最近发生了什么？**

### 9.1 桌面布局

```text
┌───────────────────────────────────────────────────────┐
│ 早上好 / 今天                                        │
│ 3 个待办 · 2 个习惯 · 1 次训练计划       [快速记录] │
├────────────┬────────────┬────────────┬────────────────┤
│ 今日进度   │ 本周训练   │ 本月支出   │ 阅读 / 学习    │
├───────────────────────────────┬───────────────────────┤
│ 今天的行动                    │ 本周趋势              │
│ · 任务                        │ compact chart         │
│ · 习惯                        │                       │
│ · 时间块                      │                       │
├───────────────────────────────┼───────────────────────┤
│ 最近动态                      │ 快速入口              │
│ timeline                      │ 记账 / 笔记 / 训练等  │
└───────────────────────────────┴───────────────────────┘
```

### 9.2 Dashboard 规则

- 第一屏 KPI 最多 4 个；
- KPI 必须有明确时间语义，例如“本周训练”；
- 不展示无法产生下一步行动的指标；
- 趋势图优先展示 7 天 / 30 天；
- 最近动态统一跨业务时间线；
- 快捷入口最多 4–6 个；
- 支持 Dashboard Card 后续个性化，但本轮不必实现拖拽布局。

### 9.3 Dashboard 图表

第一阶段只允许 3 类：

- Line / Area：连续趋势；
- Bar：周期比较；
- Donut / Radial：完成率或组成。

禁止为了视觉效果引入难读的 Radar、3D Chart 或复杂混合图。

---

## 10. 页面级重构规范

### 10.1 计划与待办

参考 Linear / shadcn Data Table 的紧凑感。

桌面端：

```text
左：列表 / Filter
中：任务内容
右：Detail Drawer（必要时）
```

重点：

- Inbox、Today、Upcoming、Project 使用一致导航；
- 快速新增支持键盘；
- 行项目优先，不把每个任务变成大 Card；
- 状态、优先级、日期使用 Badge；
- 编辑详情使用 Drawer / Sheet，减少页面跳转。

### 10.2 日历

- 周视图作为桌面主视图；
- 月视图用于概览；
- 手机端默认 Agenda；
- 任务 / 习惯 / 训练用有限的语义颜色区分；
- 不使用彩虹色事件系统。

### 10.3 习惯

首页表达：

- 今日完成状态；
- 连续天数；
- 最近 7 / 30 天完成率。

详情：

- Heatmap；
- Trend；
- Log list。

### 10.4 健身

视觉可适当参考 Apple Health，但保持 LifeTrace 自身颜色体系。

推荐结构：

```text
本周摘要
训练次数 / 总时长 / 总容量

趋势
[训练次数] [容量] [时长]

最近训练
列表

训练详情
动作 → 组 → 重量 / 次数
```

### 10.5 财务

财务页更接近数据产品，不做成银行 App 仿品。

一级信息：

- 本月支出；
- 本月收入；
- 净流入；
- 账户余额。

二级信息：

- 30 天趋势；
- 分类占比；
- 最近流水。

流水使用 Data Table / compact list，支持 Filter 和 Search。

隐私金额继续复用现有全局 Privacy 状态。

### 10.6 笔记

桌面端优先采用：

```text
列表 + 编辑器
```

而不是大量卡片铺满页面。

- 左侧笔记列表；
- 右侧正文；
- 搜索与标签在列表顶部；
- 收藏/置顶只使用小图标和状态；
- 阅读页面产生的笔记应能快速回到来源。

### 10.7 英语学习

- 阅读列表与阅读器明确分层；
- 阅读器最大正文宽度约 680–760px；
- 生词、高亮、快捷笔记使用右侧 Drawer 或浮动工具条；
- 完成阅读后给出清晰但克制的完成反馈；
- 数据统计不挤占阅读正文区域。

### 10.8 复盘

强调文本和时间结构，不做复杂仪表盘。

推荐：

- Daily Review；
- Weekly Summary；
- Streak / Completion；
- 历史列表。

### 10.9 设置

完全采用成熟 Application UI 模式：

```text
左侧 Settings Nav
右侧 Settings Content
```

分类：

- Profile；
- Appearance；
- Sync；
- Devices；
- Privacy；
- Data；
- About。

### 10.10 登录 / 注册

保持简单：

- 居中窄表单；
- 不使用夸张营销 Hero；
- 品牌 Logo + 一行产品定位；
- Login / Register / Error / Loading 使用同一布局。

---

## 11. 响应式策略

不按“桌面 CSS 打补丁”的方式做移动端。

建议断点：

```text
< 640px       Mobile
640–1023px    Tablet
>= 1024px     Desktop
>= 1440px     Wide Desktop
```

### 11.1 Desktop

- 左 Sidebar；
- 页面最大内容宽度 1440–1480px；
- Dashboard 允许双栏；
- Detail Drawer 从右侧打开。

### 11.2 Tablet

- Sidebar 默认折叠；
- Dashboard 2 列 KPI；
- 主/次栏根据内容切成单列或 2:1；
- 大表格允许横向滚动。

### 11.3 Mobile

底部只保留 4–5 个最高频入口，其余放入“更多”。

建议：

```text
今日
计划
记录
数据
更多
```

移动端：

- 不展示桌面 Sidebar；
- 页面 Header 更紧凑；
- Primary Action 可放 FAB 或底部 Action；
- Data Table 自动切换为 compact list；
- Dialog 优先变为 Sheet / Drawer；
- Calendar 默认 Agenda；
- 双栏编辑器改为列表页 → 编辑页。

所有可点击区域至少满足移动端舒适触控尺寸。

---

## 12. 动效规范

### 12.1 允许

- 页面内容 120–180ms fade / translate；
- Drawer / Dialog；
- Hover；
- Progress 数值变化；
- 成功操作短反馈；
- Dashboard 数字轻量滚动。

### 12.2 禁止

- 常驻粒子背景；
- 卡片持续发光；
- 页面大范围视差；
- 高强度弹性动画；
- 影响输入速度的过渡；
- 页面切换超过约 250ms 的非必要动画。

必须继续支持 `prefers-reduced-motion`。

---

## 13. Accessibility

重构过程中必须保持或提升：

- `focus-visible`；
- 完整键盘导航；
- Dialog Focus Trap；
- Tooltip 不承载唯一关键信息；
- 图标按钮必须有 `aria-label`；
- 颜色不能作为唯一状态表达；
- 图表必须有文字摘要；
- 正文与背景达到合理对比度；
- 表单错误与字段建立语义关联；
- 移动导航提供 `aria-current`。

---

## 14. CSS 与组件迁移策略

当前入口按顺序加载：

```text
styles.css
cloud-pages.css
web-tokens.css
web-primitives.css
web-shell.css
web-auth.css
web-workspaces.css
web-beecount.css
web-features.css
web-photo-challenge.css
```

最终目标是让旧 `styles.css` 不再承担 Web 主设计系统职责。

### 14.1 迁移原则

每迁移一个页面：

1. 找到其 legacy selector；
2. 使用新 UI Primitive / Layout 重写 JSX；
3. 页面 CSS 只保留真正业务独有的布局；
4. 删除已无引用的旧 selector；
5. 在 Light / Dark / Desktop / Mobile 下验收；
6. 再进入下一页面。

禁止只在文件底部继续堆叠 override。

### 14.2 命名

新组件不继续扩散历史 `.hx-*` 前缀。

React 组件优先表达语义：

```text
<Button />
<Card />
<PageHeader />
<MetricCard />
<ChartCard />
<DataTable />
<EmptyState />
```

若仍需全局 class，统一 `lt-` 前缀。

---

## 15. 推荐目录结构

目标结构：

```text
apps/desktop/web-client/src/
├── components/
│   ├── ui/
│   ├── layout/
│   ├── data-display/
│   └── feature/
├── pages/
├── hooks/
├── lib/
├── styles/
│   ├── tokens.css
│   ├── base.css
│   ├── utilities.css
│   └── legacy.css
├── App.tsx
├── navigation.ts
└── main.tsx
```

迁移完成后再决定是否物理移动现有 `web-*.css`，避免在第一阶段同时进行目录重组和 UI 改造。

---

## 16. 实施阶段

## Phase 0：基线冻结

目标：保证视觉重构前可回归。

任务：

- 记录当前所有 Route；
- 记录关键业务流程；
- 建立页面截图基线；
- 确认 Light / Dark；
- 确认 Desktop / Mobile；
- 确认 `npm run browser:build` 与现有单测通过。

输出：

- Route inventory；
- 页面迁移矩阵；
- visual baseline。

## Phase 1：Design System

任务：

- 整理 `web-tokens.css`；
- 建立新 `components/ui`；
- 统一 Button / Input / Card / Badge / Dialog / Tabs / Dropdown / Tooltip；
- 引入 Tailwind + shadcn（如执行时仍采用本方案）；
- 建立 Light / Dark Token；
- 统一 Skeleton / Empty / Error / Toast。

验收：

- 新页面不再自行实现通用控件；
- 组件有清晰 Variant；
- Dark Mode 无硬编码白底/黑字遗漏。

## Phase 2：App Shell

任务：

- 拆分 `AppShell.tsx`；
- 重做 Sidebar；
- 重做 Page Header；
- Command Search；
- Mobile Bottom Navigation；
- 移动端 Sheet Navigation；
- Sync / Privacy / Account 状态统一。

这是所有页面迁移的前置条件。

## Phase 3：Dashboard

任务：

- 重构首屏信息层级；
- KPI 收敛到 4 个以内；
- 今日行动；
- 最近动态；
- 快捷操作；
- 引入第一批统一 Chart；
- 完成 Desktop / Tablet / Mobile。

Dashboard 通过后作为其他页面的视觉基准。

## Phase 4：核心高频页面

顺序：

```text
计划与待办
→ 日历
→ 习惯
→ 健身
→ 财务
```

原则：一页迁移完整再进入下一页，避免全仓库同时出现半成品。

## Phase 5：内容与学习页面

顺序：

```text
笔记
→ 英语
→ 复盘
→ 搜索
```

重点验证阅读体验和移动端输入体验。

## Phase 6：系统页面

顺序：

```text
设置
→ 设备
→ 云端状态
→ 登录注册
→ 其他低频页面
```

## Phase 7：Legacy 清理

任务：

- 删除无引用 selector；
- 合并重复 CSS；
- 删除仅用于历史设计的样式文件；
- 清理旧组件；
- 清理重复图标和重复布局；
- 检查 bundle；
- 更新文档。

---

## 17. 页面迁移优先级

### P0

- App Shell；
- Dashboard；
- 通用组件；
- Responsive；
- Theme；
- Auth。

### P1

- Execution；
- Calendar；
- Habits；
- Fitness；
- Finance；
- Notes。

### P2

- English；
- Review；
- Search；
- Devices；
- Settings；
- 低频工具页。

---

## 18. 测试策略

每个 Phase 至少执行：

```bash
npm run lint
npm run test:unit
npm run browser:build
```

如果仓库已有更高层测试入口，则继续执行：

```bash
npm run test:desktop
npm run test:all
```

### 18.1 UI 验收矩阵

至少覆盖：

| 维度 | 场景 |
|---|---|
| Theme | Light / Dark |
| Width | 375 / 768 / 1024 / 1440+ |
| Session | 登录 / 未登录 |
| Network | Online / Offline |
| Data | Empty / Normal / Large |
| Privacy | 金额显示 / 隐藏 |
| Sync | Normal / Loading / Conflict / Error |
| Motion | Normal / Reduced Motion |

### 18.2 关键回归流程

- 登录、退出；
- Dashboard 正常加载；
- 新建 / 编辑 / 完成任务；
- 日历查看；
- 习惯打卡；
- 训练记录查看；
- 财务流水查看及金额隐私；
- 笔记编辑；
- 英语阅读；
- 搜索；
- 切换主题；
- 切换离线；
- 刷新同步；
- 手机导航。

---

## 19. Definition of Done

Web UI 重构完成必须满足：

- [ ] 所有现有 Web Route 可正常访问；
- [ ] 业务能力无功能性回退；
- [ ] Dashboard 完成新信息架构；
- [ ] Desktop / Tablet / Mobile 均为主动设计，而非简单缩放；
- [ ] Light / Dark 使用统一语义 Token；
- [ ] 通用控件已经组件化；
- [ ] 新页面不再新增重复按钮、卡片、Dialog 样式；
- [ ] 核心数据图表统一使用同一 Chart 体系；
- [ ] 全局搜索具备明确入口；
- [ ] 金额隐私状态保持可用；
- [ ] 在线 / 离线 / 同步 / 冲突状态保持可用；
- [ ] 键盘焦点与基础 Accessibility 验收通过；
- [ ] `prefers-reduced-motion` 生效；
- [ ] 旧 `styles.css` 中与新系统冲突的规则已经清除；
- [ ] 无大面积 override 堆叠；
- [ ] `npm run browser:build` 通过；
- [ ] 相关测试通过；
- [ ] 文档与最终组件结构同步更新。

---

## 20. 最终决策摘要

LifeTrace Web 端采用以下方向：

```text
基础组件与 Token
    shadcn/ui 风格

应用布局与信息层级
    Catalyst / Tailwind Application UI 风格

数据可视化
    Tremor 视觉理念
    + Recharts / shadcn Charts 实现

动效
    Magic UI / Aceternity 仅局部借鉴

品牌表达
    LifeTrace 低饱和绿色 + 中性背景
```

真正要实现的不是“把几个模板拼起来”，而是将这些成熟设计体系中适合 LifeTrace 的部分收敛为 **一套自己的 Web Design System**。

重构时始终遵循：

```text
先统一系统
再统一页面
先保证业务
再追求视觉
少而精的组件
少而清晰的层级
少而必要的动效
```
