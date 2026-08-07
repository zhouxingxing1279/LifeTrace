# LifeTrace Design System

目标：一套可持续 2~3 年的统一桌面软件视觉体系。风格参照 Linear、Notion、Raycast、macOS 原生应用的**信息架构与交互成熟度**，不做像素级模仿，不复制 SaaS Landing Page 或 AI Dashboard 风格。

## 1. Design Tokens

所有 token 定义在 `apps/desktop/app/tokens.css`，页面与组件禁止硬编码 UI 数值。

### 1.1 颜色（语义化）

| Token | 用途 |
| --- | --- |
| `--ui-bg-app` | 应用底色 |
| `--ui-bg-surface` | 面板 / 卡片 / 弹窗表面 |
| `--ui-bg-subtle` / `--ui-bg-hover` / `--ui-bg-active` | 次级表面与悬停状态 |
| `--ui-border` / `--ui-border-strong` | 分隔线与输入边框 |
| `--ui-foreground` / `--ui-muted` / `--ui-faint` | 主文字 / 次要文字 / 弱化文字 |
| `--ui-primary` | 唯一主强调色（操作、选中、进度） |
| `--ui-success` / `--ui-warning` / `--ui-danger` / `--ui-info` | 语义状态色 |
| `--ui-focus` / `--ui-focus-ring` | 键盘焦点 |

规则：

- 全应用只有一个主强调色。
- 禁止大面积渐变、发光、毛玻璃、彩色卡片墙。
- 深色主题通过 `html[data-theme="dark"]` 切换，同一组 token 名换值。

### 1.2 字体

| 层级 | 字号 / 行高 |
| --- | --- |
| Page Title | 24px / 32px |
| Section Title | 18px / 26px |
| Card Title | 16px / 24px |
| Body | 14px / 21px |
| Secondary | 13px / 19px |
| Caption | 12px / 17px |
| Micro（仅极低优先级） | 11px / 15px |

规则：

- 正文不得低于 14px；12px 只用于低优先级信息。
- 同一页面字体层级不超过 4 层。
- 禁止用缩小字号解决布局问题。

### 1.3 间距

只使用 scale：`4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48`，对应 `--ui-space-1` ~ `--ui-space-9`。禁止随机 margin / padding（如 13px、17px、19px）。

### 1.4 圆角

| 场景 | 值 |
| --- | --- |
| 小控件 | 6px（`--ui-radius-sm`） |
| 按钮 / 输入框 | 8px（`--ui-radius-md`） |
| 面板 / 卡片 | 10px（`--ui-radius-lg`） |
| 弹窗 | 12px（`--ui-radius-xl`） |

禁止所有元素一律 16~20px 大圆角、禁止无意义胶囊设计。

### 1.5 阴影 / 层级 / 动效

- 阴影：`--ui-shadow-sm/md/lg/menu/dialog/popover`，面板默认无阴影或仅 sm。
- 层级：`--ui-z-sticky/menu/dialog/toast`。
- 动效：`--ui-transition-fast/base/slow`，尊重 `prefers-reduced-motion`。

## 2. 布局

桌面优先，重点窗口尺寸 1280×720、1366×768、1920×1080、2560×1440。

- 侧边栏 `--ui-sidebar-width: 232px`，支持折叠至 60px。
- 内容区 `--ui-content-max: 1440px`，页面内容居中。
- 窗口缩小时侧边栏变为抽屉，表格允许横向滚动，工具栏可收缩。
