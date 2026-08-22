# LifeTrace Web Frontend V2 Clean-room Rewrite

> 状态：V2 / 执行规范  
> 适用范围：`apps/web`  
> 权威性：本文件替代旧版 Web 重构方案，作为后续 Codex / Agent 执行 Web 前端重写的唯一主规范。

---

## 1. 目标与原则

LifeTrace Web V2 不再进行“基于现有前端继续重构、换肤、迁移组件或逐页改造”。

本次工作定义为 **Clean-room Frontend Rewrite（隔离式前端重写）**：

1. 在独立 Git 分支中工作；
2. 为当前 `main` 保留可恢复基线；
3. 删除 `apps/web` 现有前端实现；
4. 禁止从 Git 历史恢复、复制或参考旧 Web 前端代码；
5. 仅从后端能力、API 契约、业务文档、数据模型和明确需求重新推导产品功能；
6. 先建立信息架构与 Design System，再实现业务页面；
7. 所有模块完成、测试通过后才允许合并回 `main`。

本次不是 refactor，而是 rewrite。

核心产品定位：

> **LifeTrace Web 是个人管理平台 / Personal OS / Productivity Application，不是传统 Admin Dashboard。**

V2 必须解决旧版中由历史实现造成的视觉惯性、页面结构惯性、CSS 惯性、组件复用惯性和交互惯性。

---

## 2. 重写范围

### 2.1 本轮必须重写

目标目录：

```text
apps/web/
```

现有 `apps/web` 中的以下内容均视为 Legacy Frontend Implementation：

```text
src/
e2e/
index.html
package.json
vite.config.*
tailwind.config.*
postcss.config.*
tsconfig.*
Playwright 配置
Web 专用脚本
旧 CI / cutover 文档
其他只服务于旧 Web 实现的文件
```

执行阶段允许在删除后重新创建同名工程文件，但必须从 V2 架构重新生成，而不是复制旧实现。

### 2.2 本轮默认不删除

除非确有接口兼容问题，本次不推倒：

```text
services/
contracts/
crates/
后端 API
数据库 / 数据模型
认证与会话协议
同步协议
LifeTrace 业务文档
apps/desktop/
apps/photo-challenge-pwa/
```

`apps/desktop` 不是本次 Web V2 clean-room rewrite 的删除对象。后续若桌面端需要统一 UI，应单独建立任务，不得顺手扩大本轮范围。

### 2.3 后端修改原则

默认复用现有后端。

只有在 V2 前端通过真实业务需求发现以下问题时才允许补后端：

- 缺失必要查询接口；
- 接口无法满足合理的分页、过滤、聚合；
- 数据模型与真实业务要求冲突；
- BeeCount 财务模块兼容需要；
- 安全、鉴权或同步协议存在明确缺口。

禁止为了方便前端重写而无理由重构后端。

---

## 3. Git 与隔离策略

### 3.1 分支

V2 必须在独立分支开发，例如：

```text
feature/frontend-v2-clean-rewrite
```

禁止直接在 `main` 上执行删除或重写。

### 3.2 基线

开始删除前必须确保：

- `main` 已同步最新状态；
- 当前 Legacy Web 可从 `main` 或基线 tag 恢复；
- 如在本地执行，推荐建立 tag：

```bash
git checkout main
git pull
git tag frontend-v1-before-clean-rewrite
git checkout -b feature/frontend-v2-clean-rewrite
```

如果已经通过远端分支开展任务，则 `main` 本身就是最低限度的恢复基线。

### 3.3 删除必须形成独立提交

建议第一阶段提交：

```text
chore(web): remove legacy frontend for v2 clean rewrite
```

删除旧前端和创建新 V2 不应混在同一个巨大提交中。

---

## 4. Clean-room 强制规则

这是 V2 最重要的约束。

### 4.1 禁止读取旧 Web 实现

在开始 V2 后，Agent **不得**通过以下方式恢复或学习旧 Web：

```text
git show <old-ref>:apps/web/...
git checkout <old-ref> -- apps/web
git restore --source=<old-ref> apps/web
git diff <old-ref> -- apps/web
git log -p -- apps/web
git blame apps/web/...
```

也不得：

- 从旧 PR、旧 commit patch 复制 `apps/web`；
- 从缓存、构建产物或历史压缩包恢复旧 Web；
- 将旧组件改名后继续使用；
- 将旧 CSS / Tailwind class 体系搬入新工程；
- 以“兼容旧页面”为理由保留旧布局；
- 把旧 UI 当视觉参考。

### 4.2 允许读取的事实来源

Agent 可以并且应该读取：

```text
services/
contracts/
crates/
docs/
README.md
LifeTrace_Future_Requirements.md
后端路由 / Controller / Service
API schema
数据库模型
测试用例中的业务约束
明确的产品需求
BeeCount Web 源码（仅财务模块）
```

### 4.3 功能与 UI 必须解耦

旧 Web 中即使存在正确业务功能，也不能因为旧实现存在就复制 UI。

正确流程是：

```text
后端 / 文档 / 契约
        ↓
业务能力清单
        ↓
用户任务与信息架构
        ↓
页面模型
        ↓
Design System
        ↓
全新实现
```

而不是：

```text
旧页面
 ↓
换 CSS
 ↓
V2
```

### 4.4 Agent 规则文件

实际执行重写时，应在分支根目录建立或更新 `AGENTS.md`，至少加入：

```text
Frontend V2 is a clean-room rewrite.
Do not inspect, restore, copy, or derive implementation from historical apps/web code.
Do not use git history as a frontend implementation reference.
Derive behavior from backend APIs, contracts, docs, tests, and explicit requirements.
The legacy Web UI must be treated as nonexistent.
```

该约束在 V2 完成前持续有效。

---

## 5. 设计目标

### 5.1 产品气质

V2 应接近成熟的 Productivity Software：

```text
Linear
Raycast
Notion / Notion Calendar
Vercel Dashboard
Apple 系统级生产力应用的信息克制程度
```

这些名称用于描述产品气质，不替代后续指定的模板参考体系。

必须体现：

- 低饱和视觉；
- 清晰 typography hierarchy；
- 高信息密度但不拥挤；
- 有边界但不过度卡片化；
- 大量功能通过列表、分区、工具栏、详情面板表达；
- 强调“当前行动”和“信息浏览效率”；
- 动效服务于状态反馈，而不是装饰；
- Desktop / Tablet / Mobile 均为正式形态。

### 5.2 明确禁止的视觉模式

除非某一业务场景有充分理由，不允许：

```text
巨大 Hero
营销型 Dashboard
紫色 / 彩色大渐变背景
Glassmorphism 全站化
每个区块都套 Card
四到八个 KPI Card 平铺成墙
重阴影
无意义的大圆角容器嵌套
Emoji 作为主要 UI 图标
“Welcome back 👋”式模板首页
发光边框 / 粒子背景大面积使用
为了炫技加入复杂动画
```

V2 必须看起来像长期使用的软件，而不是 SaaS Landing Page 或 AI UI demo。

---

## 6. 指定前端模板参考体系

V2 继续使用最初确定的优秀前端模板 / UI 体系作为实现参考：

1. **shadcn/ui**
2. **Tremor**
3. **Preline UI**
4. **Tailwind Plus / Catalyst**
5. **Aceternity UI**
6. **Magic UI**
7. **Shadcnblocks**
8. **TailAdmin**

这些项目是 **布局、交互、组件组合、信息层级和数据呈现的参考源**，并不代表必须全部安装。

### 6.1 参考职责

| 参考源 | V2 主要用途 |
|---|---|
| shadcn/ui | Primitive、Form、Dialog、Sheet、Command、Table、基础交互规范 |
| Catalyst | App Shell、Sidebar、Settings、List、Detail、成熟应用布局 |
| Shadcnblocks | Dashboard、Todo、Calendar、Settings 等页面级组合 |
| Tremor | Analytics、Trend、KPI、图表与数据表达 |
| Preline UI | 响应式页面、Account、Form、Dashboard Pattern |
| TailAdmin | 高密度表格和复杂信息页面备用参考 |
| Magic UI | 少量完成反馈、数字变化、微交互 |
| Aceternity UI | AI 区域或少数重点入口的局部视觉增强 |

### 6.2 决策优先级

```text
业务可用性
> 信息架构
> Design System 一致性
> shadcn/ui 基础交互
> Catalyst Application Layout
> Shadcnblocks / Preline 页面组合
> Tremor 数据表达
> TailAdmin 高密度信息组织
> Magic UI / Aceternity 装饰
```

### 6.3 页面参考映射

每个核心页面编码前，Agent 应记录类似：

```text
Page: /app/today
Primary page pattern: Shadcnblocks
Application shell: Catalyst
Primitive interaction: shadcn/ui
Analytics: Tremor
Motion: none unless required
```

目的不是像素复制，而是防止不同页面由 Agent 自由发挥后形成多套设计语言。

---

## 7. V2 信息架构

Agent 在编码之前必须首先根据业务能力完成功能 inventory，再验证下列 IA 是否覆盖实际能力。

建议顶层导航：

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

若实际后端能力与此不符，应根据真实业务调整，而不是为了保留旧路由强行兼容。

### 7.1 Today

Today 是行动中心，而不是 KPI 展示墙。

只回答：

1. 今天最重要的事情是什么；
2. 今天计划完成什么；
3. 已经完成什么；
4. 下一步应该做什么；
5. 是否存在值得关注的趋势或异常。

推荐组成：

```text
Date + primary actions
Today Focus
Tasks / Habits
Schedule
Recent / Important Signals
Quick Capture
```

首屏最多保留少量真正必要的数字。

### 7.2 Plan / Tasks

参考：Shadcnblocks Todo + Catalyst List/Detail。

建议：

```text
Inbox
Today
Upcoming
Projects
Completed
```

Desktop 优先 List + Detail Panel / Drawer，Mobile 使用 List → Detail。

### 7.3 Calendar

至少考虑：

```text
Month
Week
Day
Agenda
```

Mobile 优先 Day / Agenda，不允许简单压缩 Desktop Month View。

### 7.4 Habits

核心是：

```text
Today check-in
Streak
7 / 30 day trend
Heatmap
Completion rate
```

趋势是辅助，不应让图表淹没当天打卡任务。

### 7.5 Fitness / Health

核心结构：

```text
This Week
Recent Workouts
Training Volume
Exercise / Muscle Distribution
Body / Health Metrics
Trend
```

采用数据密集但克制的分析布局。

### 7.6 Finance

财务属于特殊模块，详见第 10 节。

### 7.7 Reading / English

阅读器必须内容优先。

进入阅读状态后主动弱化 App Shell，保证：

- 正文阅读；
- 高亮；
- 快捷笔记；
- 已读状态；
- 阅读完成反馈；
- 历史笔记可重新打开；
- 学习历史和必要统计。

### 7.8 Notes

Desktop 优先：

```text
Notes List | Editor
```

需要组织层时可以扩展：

```text
Folders | Notes | Editor
```

不要把笔记做成卡片瀑布流作为唯一形态。

### 7.9 Review

支持日 / 周 / 月级回顾时，应突出：

- 已完成事项；
- 关键趋势；
- 异常；
- 主观总结；
- 下一周期行动。

### 7.10 Settings

建议：

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

### 7.11 Auth

只做成熟应用登录界面，不制作营销 Landing Page。

### 7.12 AI

AI 助手优先采用 Command / Dialog / Side Panel / Inline Assistant，而不是单独制造一套发光视觉体系。

---

## 8. Design System 必须先于业务页面

任何业务页面大规模开发前，必须先建立 V2 Design System。

### 8.1 Design Tokens

至少定义：

```text
Color
Typography
Spacing
Radius
Border
Shadow
Motion
Breakpoint
Z-index
Chart palette
Semantic status
```

颜色必须语义化，而不是页面内随意写颜色值。

推荐语义：

```text
background
foreground
surface
surface-subtle
muted
muted-foreground
primary
primary-foreground
secondary
accent
destructive
success
warning
info
border
input
ring
income
expense
chart-1 ... chart-n
```

### 8.2 视觉约束

- 主体 neutral；
- LifeTrace 品牌色只用于主操作、选择态和必要强调；
- 默认优先 border / spacing 区分区域，而不是阴影；
- Card 只在“这确实是独立对象”时使用；
- 圆角层级必须固定，禁止组件各自决定；
- 字号、字重、行高必须形成固定层级；
- 图表颜色必须统一；
- Light / Dark 如实现，必须共享语义 token。

### 8.3 Primitive 组件

至少建立：

```text
Button
IconButton
Input
Textarea
Select
Checkbox
Radio
Switch
Badge
Tabs
Table / DataTable
Dialog / AlertDialog
Sheet / Drawer
DropdownMenu
Popover
Tooltip
Command
Toast
Skeleton
Separator
ScrollArea
Progress
EmptyState
ErrorState
PageHeader
SectionHeader
```

### 8.4 禁止页面私有设计系统

业务 feature 可以组合组件，但不得重复实现：

```text
自己的 Button
自己的 Modal
自己的 Toast
自己的 Badge
自己的颜色 Token
自己的 Typography 体系
```

所有模块必须共享一个 V2 设计语言。

---

## 9. V2 前端架构

重新初始化后的建议目录：

```text
apps/web/
├── package.json
├── index.html
├── vite.config.ts
├── tsconfig.json
├── public/
├── e2e/
└── src/
    ├── app/
    │   ├── router/
    │   ├── providers/
    │   └── bootstrap/
    ├── layouts/
    ├── components/
    │   ├── ui/
    │   ├── navigation/
    │   ├── data-display/
    │   └── feedback/
    ├── features/
    │   ├── today/
    │   ├── planning/
    │   ├── calendar/
    │   ├── habits/
    │   ├── fitness/
    │   ├── health/
    │   ├── finance/
    │   ├── reading/
    │   ├── notes/
    │   ├── review/
    │   ├── search/
    │   └── settings/
    ├── services/
    ├── hooks/
    ├── stores/
    ├── lib/
    ├── types/
    └── styles/
```

### 9.1 推荐技术栈

优先：

```text
React 19
TypeScript
Vite
React Router
Tailwind CSS
shadcn/ui pattern
Lucide React
Recharts / 与 BeeCount 兼容的图表方案
Zustand（仅必要客户端状态）
Vitest
Testing Library
Playwright
```

具体依赖版本应以执行时稳定版本、仓库约束和 BeeCount 复用兼容性为准，不为追求最新版而引入额外迁移风险。

### 9.2 状态边界

- Server state 不应无意义搬进全局 Zustand；
- Feature local state 优先本地；
- 全局 store 只保存真正跨页面共享的客户端状态；
- API client 必须集中管理 auth、error、base URL 和序列化策略；
- 页面不得自行散落实现 fetch 规则。

### 9.3 App Shell

Desktop：

```text
Sidebar + Main Workspace
```

应支持：

- 清晰主导航；
- 合理 collapsed 模式；
- Page Header；
- Command Palette (`Ctrl/Cmd + K`)；
- Search；
- 快速创建 / Capture；
- 状态反馈。

Mobile：

```text
Bottom Navigation + More Sheet / Contextual Header
```

Mobile 不是把 Sidebar 隐藏后结束。

---

## 10. Finance：BeeCount Web 源码复用例外

财务模块不是 clean-room UI 规则的普通对象。

已确定策略：

> **财务业务交互优先复用 BeeCount Web 的成熟实现，不重新从零仿写。**

允许 Agent 阅读 BeeCount Web 源码，并将其作为财务功能事实和交互实现来源。

但复用后必须完成 LifeTrace V2 适配：

- 接入 LifeTrace 路由；
- 接入 LifeTrace 鉴权 / API adapter；
- 接入统一 Design Tokens；
- 统一 Typography；
- 统一导航与 Page Shell；
- 统一 Dialog / Toast / Sheet 等公共 Primitive；
- 保持 BeeCount 的成熟财务交互，不无理由重新设计核心记账流程。

推荐结构：

```text
features/finance/
├── beecount/
├── adapters/
├── routes/
└── integration/
```

原则：

```text
BeeCount domain interaction
        +
LifeTrace V2 visual system
        +
LifeTrace backend compatibility
```

而不是把 BeeCount 页面以完全不同风格直接塞进 LifeTrace。

所有第三方 / 开源许可、作者信息和 notice 必须保留。

---

## 11. Codex / Agent 执行顺序

禁止“一次提示词直接生成全站然后收尾”。

Codex 可以连续自主执行，但必须遵守阶段门禁。

### Phase 0：基线与隔离

- 确认分支不是 `main`；
- 确认 `main` 可恢复；
- 建立 clean-room Agent 规则；
- 记录当前后端 / contracts / docs 可用来源。

### Phase 1：删除 Legacy Web

- 删除当前 `apps/web` 实现；
- 不恢复旧源码；
- 形成独立提交。

门禁：仓库其他应用和后端未被误删。

### Phase 2：产品能力 Inventory

Agent 必须从允许来源梳理：

```text
Feature
User goal
Backend endpoint
Data model
Required states
Create / Read / Update / Delete capability
Cross-module dependency
Auth requirement
Responsive requirement
```

产出 V2 功能清单，并与本文 IA 对账。

门禁：不能依赖旧 `apps/web` 获取功能清单。

### Phase 3：信息架构与 UX Skeleton

先定义：

- 顶层导航；
- 路由；
- 页面职责；
- Desktop / Tablet / Mobile 行为；
- List / Detail / Modal / Drawer 关系；
- Loading / Empty / Error / Permission State。

禁止此时大量写视觉样式。

### Phase 4：Design System

完成：

- Token；
- Typography；
- Primitive；
- App Shell；
- Navigation；
- Page layout primitives；
- Feedback primitives。

门禁：至少使用 Story / test page / real shell 验证组件一致性后再铺业务页。

### Phase 5：核心工作区

优先实现：

1. Auth；
2. App Shell；
3. Today；
4. Plan / Tasks；
5. Calendar；
6. Search / Command。

这些页面先确定整个产品的交互密度和布局基准。

### Phase 6：业务模块

建议顺序：

```text
Habits
Fitness / Health
Reading / English
Notes
Review
Settings
```

每完成一个模块立即对齐 Design System，禁止多个 Agent 各自创造 UI。

### Phase 7：Finance Integration

- 导入 / 复用 BeeCount Web；
- 建 adapter；
- 对接 LifeTrace；
- 统一视觉；
- 跑财务核心回归。

### Phase 8：Responsive & Polish

逐页验证：

```text
Desktop
Tablet
Mobile
Keyboard
Touch
Long content
Empty state
Error state
Loading
Large data set
```

### Phase 9：测试与收口

必须执行并修复：

```text
lint
typecheck
unit tests
integration tests
e2e
production build
```

如果仓库已有额外 CI 门禁，也必须通过。

---

## 12. 测试要求

### 12.1 功能测试

核心业务必须覆盖真实用户路径，而不是只测组件 render。

最低 E2E 建议：

```text
login
navigate major modules
create / edit / complete task
habit check-in
open calendar / change view
record or inspect fitness data where supported
open reading item / highlight / note / complete reading
create / edit note
open review
finance core workflow
settings update
logout
```

具体路径以真实后端能力为准。

### 12.2 UI 状态测试

所有核心页面至少处理：

```text
Loading
Empty
Success
Partial data
Error
Unauthorized / expired session
Long text
Large list
```

### 12.3 响应式门禁

至少验证典型：

```text
Mobile ~ 390px
Tablet ~ 768-1024px
Desktop >= 1280px
```

不允许只保证 Desktop 截图好看。

### 12.4 可访问性

至少确保：

- keyboard focus 可见；
- 表单 label 正确；
- Dialog / Sheet focus 管理正确；
- icon-only action 有 accessible name；
- 颜色不是唯一状态表达；
- 基本对比度合理；
- reduced motion 不影响使用。

---

## 13. 代码质量与反模式

禁止：

```text
一个超大 App.tsx 承担全部路由
页面内到处直接 fetch
复制粘贴 Primitive
任意 hardcoded color
CSS specificity 战争
大量 !important
为了“好看”引入多套 UI library
单页引入另一套 design language
无测试的大范围视觉重写
为了兼容旧代码保留 legacy wrapper
```

优先：

```text
feature-oriented architecture
明确数据边界
统一 API client
组合式组件
可预测路由
语义 token
小而稳定的 primitives
按用户路径测试
```

---

## 14. Git 提交建议

建议保持可审计的提交序列：

```text
chore(web): remove legacy frontend for v2 clean rewrite
chore(web): initialize v2 application architecture
feat(web): establish v2 design system
feat(web): implement application shell and navigation
feat(web): implement today workspace
feat(web): implement planning and calendar
feat(web): implement habits
feat(web): implement fitness and health
feat(web): implement reading and notes
feat(web): integrate beecount finance
feat(web): implement review and settings
feat(web): complete responsive experience
test(web): add v2 integration and e2e coverage
docs(web): finalize v2 architecture and migration notes
```

避免最终只有一个无法审查的 `rewrite frontend` 巨型 commit。

---

## 15. Definition of Done

只有满足以下全部条件，V2 才算完成：

- [ ] 在独立分支完成开发；
- [ ] Legacy `apps/web` 已被删除而非继续演化；
- [ ] 未从 Git 历史恢复旧 Web 实现；
- [ ] 功能清单由后端、contracts、docs、tests 推导；
- [ ] 新 Design System 已建立；
- [ ] 所有业务模块使用同一设计语言；
- [ ] Finance 已按要求复用 BeeCount 并完成 LifeTrace 适配；
- [ ] Desktop / Tablet / Mobile 均完成；
- [ ] Loading / Empty / Error 等状态完整；
- [ ] 核心业务 API 已真实接入；
- [ ] lint 通过；
- [ ] typecheck 通过；
- [ ] unit / integration tests 通过；
- [ ] E2E 通过；
- [ ] production build 通过；
- [ ] 无已知 Blocker / Critical regression；
- [ ] 文档同步更新；
- [ ] 最终 diff 已确认没有误改后端或其他应用；
- [ ] 满足以上条件后才允许合并 `main`。

---

## 16. Codex 主执行指令

后续可以将下面内容作为 Codex 执行该文档时的核心指令：

```text
Implement docs/ui/web-redesign.md completely.

This is a clean-room rewrite of LifeTrace Web, not a refactor.

Work only on a dedicated feature branch. Never perform destructive frontend rewrite work directly on main.

Treat the existing apps/web implementation and every historical version of apps/web as nonexistent.

Do not inspect, restore, copy, diff against, or derive implementation from historical frontend code using git history, old commits, old PR patches, cached artifacts, or legacy bundles.

You may inspect backend code, contracts, data models, documentation, tests, explicit requirements, and BeeCount Web only for the finance integration described in the specification.

First remove the legacy Web implementation in an isolated commit. Then derive a complete feature inventory from allowed sources before rebuilding the frontend.

Establish information architecture, responsive behavior, design tokens, primitives, application shell, navigation, error/loading/empty patterns, and API boundaries before implementing the full set of business pages.

Use the template/reference hierarchy defined in the specification. The visual result must feel like a mature productivity application, not an admin template or marketing SaaS page.

Do not allow separate modules or subagents to invent independent design systems.

For Finance, reuse BeeCount Web's mature domain interaction and source implementation where appropriate, preserve required attribution/licenses, integrate it with LifeTrace APIs, and adapt it to the LifeTrace V2 design system.

Execute the work phase by phase but continue autonomously until the complete Web frontend is implemented.

After implementation, run lint, typecheck, unit/integration tests, E2E tests, and the production build. Fix all failures caused by the work. Update relevant documentation.

Do not declare completion until the Definition of Done in docs/ui/web-redesign.md is satisfied.
```

---

## 17. 最终验收标准

验收 V2 时，不以“和旧版功能长得一样”为标准。

真正的标准是：

> **在完全不继承旧 Web UI 实现的前提下，重新从 LifeTrace 的真实业务能力构建一套统一、成熟、高效率、可长期维护的 Personal OS Web 前端。**

如果新页面只是旧页面换了一套颜色、圆角和 Tailwind class，则本次 clean-room rewrite 失败。

如果页面漂亮但无法完整承载真实业务、移动端不可用、模块之间风格不一致或测试无法通过，同样视为失败。
