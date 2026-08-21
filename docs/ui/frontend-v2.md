# LifeTrace Frontend V2 — Unified Rewrite Plan

> 状态：V2 / 主入口文档  
> 分支：`feature/frontend-v2-clean-rewrite`

本文件是 LifeTrace V2 前端重构的总入口。

详细规范：

- 统一 Apple Productivity Design System：`docs/ui/apple-design-system.md`
- Web：`docs/ui/web-redesign.md`
- Desktop：`docs/ui/desktop-redesign.md`

其中 `apple-design-system.md` 是 Web 与 Desktop 共同遵守的**强制视觉与交互基线**。任何模板、页面实现或历史设计与其冲突时，以该规范为准。

---

## 1. 总目标

LifeTrace V2 不再把 Web 和 Desktop 当成两个独立 UI 项目。

目标架构：

```text
                    LifeTrace Frontend V2
                              │
            ┌─────────────────┴─────────────────┐
            │                                   │
     Shared Design System                Shared Feature Layer
            │                                   │
            └─────────────────┬─────────────────┘
                              │
             ┌────────────────┴────────────────┐
             │                                 │
           Web                              Desktop
     responsive shell                  Tauri desktop shell
      cloud adapters                  native/local adapters
```

原则：

> 同一个产品、同一套设计语言、同一套 feature，不同平台做合理增强。

统一视觉定位：

> **Apple Productivity UI = Minimal + Content First + Soft Depth + Floating Controls + Controlled Liquid Glass**

---

## 2. Clean-room 策略

### Web

`apps/web` 现有实现视为 Legacy Frontend，允许整体删除后从零重建。

### Desktop

Desktop 只 clean-room 删除 UI / renderer，不删除 Tauri、Local DB、文件系统、同步等平台能力。

Desktop 必须先做 capability inventory 和 boundary extraction，再移除旧 UI。

---

## 3. V2 唯一事实来源

新前端的产品功能从以下来源推导：

```text
services/
contracts/
crates/
docs/
LifeTrace_Future_Requirements.md
后端 API
数据库 / domain model
业务测试
Tauri native capability
明确产品需求
BeeCount Web（仅 Finance）
```

Legacy Web / Desktop UI 不是视觉或组件实现依据。

视觉与交互的事实来源按优先级为：

```text
docs/ui/apple-design-system.md
        ↓
Shared Design Tokens / Components
        ↓
模板参考体系
        ↓
业务页面组合
```

---

## 4. 共享层

V2 应优先形成以下共享抽象：

```text
Design Tokens
Typography
Primitive Components
Navigation patterns
Data display
Feedback states
Domain schemas
Feature components
Feature view models
Business hooks
Validation
Formatting
```

平台差异必须集中在 adapter 层。

Design Token 必须成为唯一视觉事实来源，页面不得随意硬编码新的颜色、圆角、间距、阴影、blur、animation duration。

---

## 5. 平台职责

### Web

重点：

```text
responsive
mobile/tablet/desktop
browser navigation
PWA / browser capability
cloud-first
```

### Desktop

重点：

```text
keyboard-first
window-aware
multi-pane
local-first capability
filesystem
native dialog
import/export
Tauri invoke
updater
native productivity interactions
```

Desktop 不得退化成“Web 页面套壳”。

---

## 6. 统一设计方向

### 6.1 强制设计基线

完整规则见：

```text
docs/ui/apple-design-system.md
```

核心原则：

```text
Content first
Controlled Liquid Glass
Soft depth
Neutral surfaces
One primary accent
Unified spacing
Unified radius
Semantic colors
Subtle motion
Whitespace over borders
```

Liquid Glass 只允许用于：

```text
Sidebar
Top Toolbar
Bottom Navigation
Floating Action Bar
Popover
Context Menu
Transient Control Layer
```

不得用于：

```text
文章正文
常驻内容 Card
表格主体
列表主体
统计块
编辑器正文
全页背景
```

### 6.2 模板参考体系

以下模板仍可作为成熟结构与组件组合参考：

```text
shadcn/ui
Tailwind Plus / Catalyst
Shadcnblocks
Tremor
Preline UI
TailAdmin
Magic UI
Aceternity UI
```

但模板不再定义最终视觉语言。

决策优先级：

```text
业务可用性
> Apple Productivity Design System
> Accessibility
> Shared Design System consistency
> shadcn/ui interaction patterns
> Catalyst / Shadcnblocks / Preline composition
> Tremor data expression
> TailAdmin dense layouts
> Magic UI / Aceternity decoration
```

总体产品气质允许参考：

```text
macOS productivity apps
Apple.com 的克制排版
Apple 系统控制层材质
Linear
Raycast
Notion / Notion Calendar
Vercel
```

禁止：

```text
Admin Template 感
大面积渐变
Glassmorphism 全站化
KPI Card Wall
页面级独立设计语言
旧 CSS override chain
为了兼容 Legacy DOM 妥协 V2
```

---

## 7. Design System 首批强制组件

业务页面开始前必须先建立共享组件：

```text
AppShell
Sidebar
Topbar / Toolbar

Button
IconButton
Input
Textarea
SearchField
Select
SegmentedControl
Switch
Checkbox

Card
StatCard
List
ListItem
Table
Badge
Progress

Tabs
Popover
Dropdown
ContextMenu
CommandPalette

Modal
AlertDialog
Sheet
Toast

EmptyState
Skeleton
Tooltip

ChartContainer
```

业务页面优先组合这些组件。

如果需要新增 Primitive，必须进入 Design System 层统一实现，不允许只在单一 feature 内发明新的视觉语言。

---

## 8. Finance 规则

Finance 是唯一允许显式复用外部成熟前端实现的业务模块。

策略：

```text
BeeCount Web
    ↓
抽取 Finance Feature
    ↓
适配 LifeTrace V2 Token / Shell
    ↓
Web + Desktop 共用
```

必须把 BeeCount 视觉适配到 Apple Productivity Design System，包括：

```text
Typography
Spacing
Radius
Surface
Buttons
Form controls
Popover / Modal
Navigation
Charts container
Dark mode
Focus states
```

禁止 Web、Desktop、BeeCount 三套财务 UI 并存。

---

## 9. 推荐执行顺序

```text
0. 建立 clean-room 规则与恢复基线
1. Web / Desktop capability inventory
2. Desktop native/domain boundary extraction
3. 建立 shared frontend architecture
4. 删除 Legacy Web
5. 删除 Legacy Desktop UI
6. 建立 Design Tokens / Light / Dark Theme
7. 实现共享 Primitive 与 accessibility states
8. 实现 Shared AppShell / Sidebar / Toolbar
9. 建立 responsive / window-aware breakpoint
10. 建立 visual regression 基准页面
11. 实现 Web Shell
12. 实现 Desktop Shell
13. 按 feature 同步实现核心业务
14. 接入 BeeCount Finance 并完成视觉适配
15. Desktop native enhancement
16. Responsive / window-aware polish
17. Design Review Gate
18. Unit / integration / E2E / Rust / build regression
19. 文档同步
20. 全部通过后合并 main
```

业务页面不得早于 Design System 基线完成。

---

## 10. Agent 执行约束

实际执行前应在仓库根目录建立或更新 `AGENTS.md`，明确：

```text
Frontend V2 is a clean-room rewrite.
Do not inspect or restore historical Web UI implementation.
Do not inspect or restore historical Desktop visual implementation.
Desktop native/domain contracts may be inspected only to preserve capability.
Derive functionality from backend/contracts/docs/tests/native capability.
Web and Desktop must share the V2 Design System and feature architecture.
Follow docs/ui/apple-design-system.md as the mandatory visual and interaction baseline.
Do not invent page-local colors, radius, spacing, blur, shadow, or motion systems.
Liquid Glass is for navigation/toolbars/transient controls, not content surfaces.
Finance may reuse BeeCount Web implementation but must adopt LifeTrace V2 tokens.
Do not stop at scaffolding; continue until functional regression and builds pass.
```

---

## 11. Design Review Gate

每个核心页面完成后必须检查：

```text
[ ] Content first
[ ] 仅使用共享 Design Tokens
[ ] Accent 使用克制
[ ] 无无意义的 Card Wall
[ ] 无装饰性渐变
[ ] 无装饰性重阴影
[ ] Glass 只用于控制层
[ ] Typography hierarchy 统一
[ ] Spacing / Radius 来自统一 token
[ ] Hover / Active / Focus / Disabled / Loading 完整
[ ] Keyboard navigation 合理
[ ] Touch target 合理
[ ] Dark Mode 正常
[ ] Reduced Motion 正常
[ ] Web responsive 正常
[ ] Desktop window-aware 正常
[ ] 未形成 page-local design language
```

未通过 Design Review Gate 的页面不得标记完成。

---

## 12. Definition of Done

V2 只有同时满足以下条件才允许合并 `main`：

```text
Web legacy UI removed
Desktop legacy UI removed
Desktop native capabilities preserved
Apple Productivity Design System implemented
Shared Design Tokens in production use
Shared Primitive components in production use
Core features visually and behaviorally aligned
Finance integration complete
No uncontrolled Liquid Glass usage
No legacy CSS imported
Web responsive verification complete
Desktop window/keyboard/native verification complete
Light / Dark mode verified
Accessibility and focus states verified
Reduced motion verified
Design Review Gate passed
TypeScript checks pass
Unit tests pass
Web E2E pass
Desktop smoke tests pass
Rust tests pass
Web build pass
Tauri build pass
Docs match implementation
```

最终目标不是分别获得一个“新 Web”和一个“新 Desktop”，而是建立一个真正统一的 **LifeTrace Frontend Platform V2**。
