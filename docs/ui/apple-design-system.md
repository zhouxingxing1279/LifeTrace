# LifeTrace Frontend V2 — Apple Productivity Design System

> 状态：V2 / 强制设计规范  
> 适用范围：LifeTrace Web V2 + Desktop V2 renderer  
> 配套文档：`docs/ui/frontend-v2.md`、`docs/ui/web-redesign.md`、`docs/ui/desktop-redesign.md`

---

## 1. 设计定位

LifeTrace Frontend V2 的统一视觉方向定义为：

> **Apple Productivity UI = Minimal + Content First + Soft Depth + Floating Controls + Controlled Liquid Glass**

目标不是 1:1 仿制 macOS，也不是做全站 Glassmorphism，而是吸收 Apple 系统与效率应用的层级、协调、一致性、内容优先和控制层材质逻辑，形成适合长期使用的 Personal OS。

V2 必须呈现：

```text
干净
克制
轻量
柔和
高信息密度但不拥挤
弱边界
强层级
少颜色
少阴影
少渐变
充足留白
```

明确禁止：

```text
全屏毛玻璃
所有 Card 都 backdrop-blur
彩色大渐变背景
Neon Glow
重阴影
无统一规则的圆角
赛博朋克蓝紫视觉
传统 Admin Dashboard 感
```

---

## 2. 界面分层模型

所有 Web 与 Desktop 页面都应遵循统一层级：

```text
Background
   ↓
Content Layer
   ↓
Cards / Lists / Charts / Editors
   ↓
Navigation & Control Layer
   ↓
Popover / Modal / Floating Controls
```

核心规则：

> **内容是主角，导航和控件浮在内容之上。**

Liquid Glass 仅属于导航、工具栏和瞬态控制层，不属于主要内容层。

Desktop 推荐：

```text
┌────────────────────────────────────────────────────────────┐
│                  Floating Top Toolbar                      │
├─────────────┬──────────────────────────────────────────────┤
│             │                                              │
│   Sidebar   │                 Content                      │
│             │                                              │
└─────────────┴──────────────────────────────────────────────┘
```

Desktop 可按业务增加 Inspector / Detail Pane；Web Tablet/Mobile 根据宽度退化为 Collapsible Sidebar、Sheet 或 Bottom Navigation。

---

## 3. Design Tokens 是唯一视觉事实来源

禁止页面直接创造独立视觉值。所有基础视觉参数必须通过共享 token 定义。

建议初始 token：

```css
:root {
  /* Typography */
  --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;

  /* Background */
  --bg-primary: #f5f5f7;
  --bg-secondary: #ffffff;
  --bg-tertiary: #f2f2f7;

  /* Surface */
  --surface-primary: #ffffff;
  --surface-secondary: #f5f5f7;

  /* Text */
  --text-primary: #1d1d1f;
  --text-secondary: #6e6e73;
  --text-tertiary: #86868b;

  /* Accent */
  --accent: #0071e3;
  --accent-hover: #0077ed;

  /* Semantic */
  --success: #34c759;
  --warning: #ff9f0a;
  --danger: #ff3b30;

  /* Border */
  --border-subtle: rgba(0, 0, 0, 0.06);

  /* Radius */
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 18px;
  --radius-xl: 24px;
  --radius-2xl: 32px;
  --radius-pill: 999px;

  /* Spacing */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;

  /* Glass */
  --glass-bg: rgba(255, 255, 255, 0.68);
  --glass-border: rgba(255, 255, 255, 0.42);

  /* Blur */
  --blur-sm: 12px;
  --blur-md: 20px;
  --blur-lg: 32px;
  --blur-xl: 48px;

  /* Motion */
  --duration-fast: 120ms;
  --duration-normal: 200ms;
  --duration-slow: 320ms;
  --ease-apple: cubic-bezier(.16, 1, .3, 1);
}

[data-theme="dark"] {
  --bg-primary: #000000;
  --bg-secondary: #1c1c1e;
  --bg-tertiary: #2c2c2e;

  --surface-primary: #1c1c1e;
  --surface-secondary: #000000;

  --text-primary: #f5f5f7;
  --text-secondary: #a1a1a6;
  --text-tertiary: #86868b;

  --border-subtle: rgba(255, 255, 255, 0.10);
}
```

以上是 LifeTrace 的实现基线，不宣称是 Apple 内部参数。

---

## 4. 颜色规则

颜色比例目标：

```text
90% Neutral
 8% Gray
 2% Accent / Semantic Color
```

Accent 仅用于：

```text
Primary Action
Selected State
Link
Focus
Active Indicator
```

禁止在同一页面同时大面积使用：

```text
Accent Card
Accent Sidebar
Accent Navbar
Accent Title
Accent Icon
Accent Button
```

颜色必须承担语义，不承担装饰。

成功 / 警告 / 危险 / 收入 / 支出等业务颜色通过 semantic token 表达，不允许硬编码到页面组件。

---

## 5. Typography

Web 不内置或分发 SF Pro 字体文件。统一采用系统字体栈：

```css
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
```

推荐层级：

| 类型 | 字号 | Weight |
|---|---:|---:|
| Hero（极少使用） | 48–64px | 600 |
| Page Title | 32–40px | 600 |
| Section Title | 24–28px | 600 |
| Card / Panel Title | 17–20px | 600 |
| Body | 14–16px | 400 |
| Secondary | 13–14px | 400 |
| Caption | 11–12px | 400 |

常用字重限制为：

```text
400
500
600
```

禁止依赖 700 / 800 / 900 制造层级。

---

## 6. Spacing

基础单位：`4px`。

允许的主要 spacing：

```text
4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64 / 80
```

推荐：

```text
Card padding: 20–24px
Component gap: 8–16px
Section gap: 24–32px
Desktop page padding: 32–48px
Tablet page padding: 24–32px
Mobile page padding: 16–20px
```

优先通过留白、分组和对齐表达层级，而不是通过更多边框。

---

## 7. Radius

圆角只允许来自 token 系统。

建议映射：

```text
Input: 10–12px
Button: 10–14px 或 capsule
Small Card: 14–18px
Card: 18–20px
Large Panel: 24px
Modal: 24–28px
Pill: 999px
```

禁止页面出现大量 11 / 13 / 17 / 19 / 21 / 25px 等随意半径。

---

## 8. Liquid Glass 使用边界

### 允许

```text
Top Navigation
Sidebar
Toolbar
Floating Action Bar
Bottom Navigation
Popover
Context Menu
Control Cluster
必要的 Modal / Sheet 表层
```

### 禁止

```text
文章正文
数据 Card 主体
列表内容层
表格
统计块
编辑器正文
整个页面背景
```

Web 可采用近似实现：

```css
.glass-control {
  background: linear-gradient(
    135deg,
    rgba(255,255,255,.72),
    rgba(255,255,255,.48)
  );
  backdrop-filter: blur(24px) saturate(180%);
  -webkit-backdrop-filter: blur(24px) saturate(180%);
  border: 1px solid rgba(255,255,255,.42);
  box-shadow: 0 8px 32px rgba(0,0,0,.08);
}
```

必须根据实际背景对比度、Dark Mode 与可读性调整，不能机械复制参数。

---

## 9. App Shell

### 9.1 Sidebar

Desktop 推荐：

```text
width: 240–280px
padding: 12–16px
item height: 40–44px
item horizontal padding: 12px
item radius: 10–12px
icon/text gap: 10px
```

状态：

```text
Default: transparent
Hover: subtle neutral surface
Selected: slightly stronger neutral surface
```

Selected 不允许同时出现蓝色大背景、左侧强调条、粗体和阴影。

Sidebar 重点强调分组、层级和当前位置，不强调装饰。

### 9.2 Top Toolbar

推荐高度：`56–64px`。

Desktop 可使用 sticky / floating glass toolbar；滚动内容应从控制层下方经过。

Top Toolbar 负责：

```text
Page Context
Back / Forward（如适用）
Primary page actions
Search
View switch / period switch
Contextual tools
```

禁止传统“白色 Navbar + 粗灰下边框”作为默认样式。

---

## 10. Card 与内容容器

Card 的层级主要来自：

```text
Surface contrast
Radius
Padding
Typography
Spacing
```

不依赖重阴影。

建议：

```css
.card {
  background: var(--surface-primary);
  border-radius: 20px;
  padding: 24px;
  border: 1px solid rgba(0,0,0,.04);
  box-shadow:
    0 1px 2px rgba(0,0,0,.02),
    0 4px 16px rgba(0,0,0,.025);
}
```

允许对大量常驻内容完全移除阴影。

不要把每个 section 都包成 Card；列表、分区、编辑器、日历等应使用最适合业务的信息结构。

---

## 11. Dashboard / Stat

禁止彩色统计卡墙。

统一原则：

```text
Card = neutral
Number = primary text
Secondary = gray
Chart = one accent / semantic color
Positive = success
Negative = danger
```

Today / Overview 首屏只保留真正必要的少量数字，避免 4–8 个等权 KPI 抢占注意力。

---

## 12. Buttons

Primary：

```text
height: 40px
padding: 0 18px
radius: capsule 或 12px
accent background
weight: 500
```

Secondary：neutral fill。

Ghost：transparent。

每个视图原则上只有 1–2 个高强调动作。

必须实现：

```text
hover
active
focus-visible
disabled
loading
```

点击区域：

```text
Desktop visual target: ~40px
Mobile minimum hit area: 44 × 44px
```

---

## 13. Input / Search

Input：

```text
height: 40px
padding: 0 12px
background: subtle neutral fill
border: transparent by default
radius: 10–12px
```

Focus 使用 accent ring，不依赖粗边框。

Search 是一等组件，采用弱背景、低边界、清晰 focus 状态。

---

## 14. Segmented Control

适用于：

```text
日 / 周 / 月 / 年
List / Board
Overview / Details
```

Outer 使用 subtle neutral background + 3px padding；Selected 使用 solid surface + 极轻阴影。

不要使用传统网页 Tab 的粗 underline 作为所有切换场景的统一模式。

---

## 15. Lists / Tables

常规列表优先使用组内 inset separator，而不是每行完整边框。

高密度数据才使用 Table。

List Item 必须支持：

```text
Primary label
Secondary metadata
Leading icon / avatar（可选）
Trailing value / action
Hover / selected / keyboard focus
```

不要把每一行包装成独立 Card。

---

## 16. Modal / Sheet / Popover

### Modal

```text
width: 400–560px（常规）
radius: 24–28px
padding: 24px
```

Overlay 使用轻暗化，可带极轻背景 blur。

### Popover

```text
radius: 14–16px
padding: 6px
item height: 36–40px
blur: ~32px（按性能和可读性调整）
```

Popover / Context Menu 是桌面效率工作流的重要组成部分，应支持键盘导航和 Escape 关闭。

---

## 17. Iconography

不复制或分发 SF Symbols 资源。

统一采用：

> **Lucide React 优先**

要求：

```text
线性
简单
统一 stroke
通常 18–22px
```

同一功能全平台使用相同语义图标。

---

## 18. Motion

动效必须解释状态与操作反馈，不作为装饰。

推荐范围：

```text
Hover: 120–160ms
Button: 100–140ms
Dropdown / Popover: 160–220ms
Modal / Sheet: 220–280ms
Page transition: 240–400ms
```

推荐 easing：

```css
cubic-bezier(.16,1,.3,1)
```

页面进入：

```text
opacity 0 → 1
translateY 6px → 0
~240ms
```

禁止 30–100px 的大幅飞入动画。

### Hover

禁止：

```text
translateY(-5px)
巨大 shadow
夸张 scale
```

建议仅使用微弱 surface / brightness / scale 变化。

### Active

允许：

```text
Button: scale(.97)
Icon: scale(.94–.97)
Card: 极轻 scale(.99)，仅在真正可点击时使用
```

必须尊重 `prefers-reduced-motion`。

---

## 19. Blur 与 Shadow

Blur token：

```text
12 / 20 / 32 / 48px
```

典型：

```text
Toolbar: ~20px
Sidebar: ~24px
Popover: ~32px
Modal surface: ~32–40px
```

Shadow 原则：

> 大、软、淡。

例如：

```css
box-shadow: 0 8px 30px rgba(0,0,0,.08);
```

常驻 Card 应比 Popover / Modal 使用更弱的 depth。

---

## 20. Responsive / Window-aware

V2 不做“Desktop 页面等比例缩小”。

### Desktop > 1200

```text
Sidebar + Content + Optional Inspector
```

### Tablet 768–1200

```text
Collapsible Sidebar + Content
```

### Mobile < 768

```text
Content + Bottom Glass Navigation / Sheet Navigation
```

Desktop Tauri 还必须考虑：

```text
窗口缩放
最小窗口尺寸
双栏 / 三栏阈值
Inspector collapse
键盘优先
窗口状态持久化
```

Mobile 的触控尺寸与 Desktop 的信息密度分别优化，不得只用 CSS scale。

---

## 21. Accessibility

必须同时满足：

```text
Light Mode
Dark Mode
Focus Visible
Keyboard navigation
Reduced Motion
Sufficient contrast
Semantic HTML / ARIA where required
Touch target size
Zoom / font scaling robustness
```

Glass 背景在复杂内容上必须检测实际文字与图标对比度；如果材质影响可读性，优先牺牲材质效果。

---

## 22. V2 首批组件白名单

第一阶段统一实现：

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

业务页面应优先组合这些组件。

若确实需要新 Primitive：

1. 先说明现有组件为何无法表达；
2. 在 Design System 层实现；
3. 补全 states / accessibility / responsive；
4. 禁止只在某个页面里临时发明新视觉语言。

---

## 23. Apple Productivity UI 十条硬规则

```text
1. Content first.
2. Liquid Glass only for navigation, toolbars and transient controls.
3. No decorative gradients.
4. No decorative shadows.
5. Use semantic colors.
6. Maximum one primary accent color.
7. Use a unified radius system.
8. Use a unified spacing system.
9. Animation must communicate interaction or state.
10. Prefer whitespace over borders.
```

任何页面设计评审都必须逐条检查。

---

## 24. 与现有 V2 参考体系的关系

Apple Productivity Design System 定义 **视觉原则和约束**。

原 V2 模板体系继续定义 **成熟组件和页面组合参考**：

```text
shadcn/ui             → Primitive / Interaction
Catalyst              → Application Shell / Settings / List / Detail
Shadcnblocks          → Page composition
Tremor                → Analytics / Charts
Preline UI            → Responsive patterns
TailAdmin              → Dense information fallback
Magic UI / Aceternity → Very limited micro-interaction / AI emphasis
```

决策优先级调整为：

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

任何模板如果与本规范冲突，以本规范为准。

---

## 25. Finance 适配

BeeCount Web 的业务交互与功能可以源码级复用，但视觉必须通过 LifeTrace V2 token 重新适配。

必须统一：

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

禁止把 BeeCount 原视觉主题原封不动嵌入 LifeTrace，造成产品割裂。

---

## 26. Codex / Agent 执行要求

在真正实现任何业务页面前，Agent 必须先完成：

```text
1. 建立 token 文件
2. 建立 Light / Dark theme
3. 实现首批 Primitive
4. 实现 AppShell / Sidebar / Toolbar
5. 建立交互状态和 accessibility
6. 建立 responsive / window-aware breakpoint
7. 建立 visual regression 基准页面
```

之后业务页面才允许开始。

禁止：

```text
先把页面写出来再补 Design System
复制旧 CSS
页面内部硬编码大量视觉值
不同 feature 自己定义 Button / Card / Modal
把 Liquid Glass 当通用 Card 样式
为了“Apple 感”使用未经约束的大量 blur / shadow / gradient
```

---

## 27. Design Review Gate

每个核心页面完成后至少检查：

```text
[ ] 是否内容优先
[ ] 是否只使用共享 token
[ ] 是否只有有限 accent
[ ] 是否存在不必要 Card
[ ] 是否存在不必要 border / shadow
[ ] Glass 是否只用于控制层
[ ] 字体层级是否来自统一 scale
[ ] spacing / radius 是否来自 token
[ ] Hover / Active / Focus 是否完整
[ ] Keyboard / Touch 是否可用
[ ] Dark Mode 是否正常
[ ] Reduced Motion 是否正常
[ ] Desktop / Tablet / Mobile 或窗口变化是否合理
[ ] 是否出现独立页面视觉语言
```

未通过以上检查的页面不得视为 V2 完成。

---

## 28. 最终视觉配方

LifeTrace V2 默认视觉基线：

```text
Background      #F5F5F7
Surface         #FFFFFF
Font            System / SF on Apple platforms
Primary text    #1D1D1F
Secondary text  #6E6E73
Accent          #0071E3
Sidebar         Controlled Liquid Glass
Toolbar         Controlled Liquid Glass
Cards           Solid Surface
Card radius     ~20px
Panel radius    ~24px
Button          Capsule / 12px
Shadow          Extremely subtle
Border          rgba(0,0,0,.06)
Desktop padding 32–48px
Card padding    20–24px
Grid gap        20–24px
Animation       120–280ms for most interactions
```

最终效果应介于：

> **macOS productivity app + Apple.com 的克制排版 + 新 Apple 系统控制层材质**

而不是单纯 Glassmorphism。

---

## 29. 参考来源

实现与设计评审应优先参考 Apple 官方 Human Interface Guidelines 与 Design Resources：

- Human Interface Guidelines: `https://developer.apple.com/design/human-interface-guidelines/`
- Design Resources: `https://developer.apple.com/design/resources/`

本文件中的 Web CSS 数值是 LifeTrace 的工程化实现建议，不代表 Apple 官方内部实现参数。
