# Design System

Token 定义在 `apps/desktop/app/tokens.css`，统一样式层在 `apps/desktop/app/hengxu.css`。

## 颜色

语义化 token：`--ui-bg-app`、`--ui-bg-surface`、`--ui-border`、`--ui-foreground`、`--ui-muted`、`--ui-primary`、`--ui-success`、`--ui-warning`、`--ui-danger`、`--ui-info`。

规则：

- 一个主强调色；成功 / 警告 / 错误用语义色。
- 禁止大面积渐变、发光、毛玻璃、彩色卡片墙。
- 深色主题通过 `html[data-theme="dark"]` 生效。

## 字体

- Page Title 24/32 · Section 18/26 · Card 16/24 · Body 14/21 · Secondary 13/19 · Caption 12/17。
- 正文不得低于 14px；页面字体层级不超过 4 层。

## 间距

仅用 `--ui-space-1..9`（4/8/12/16/20/24/32/40/48）。禁止随机数值。

## 圆角

小控件 6px · 按钮/输入 8px · 面板 10px · 弹窗 12px。

## 阴影与层级

阴影用 `--ui-shadow-*`；z-index 用 `--ui-z-*`；动效用 `--ui-transition-*` 并尊重 `prefers-reduced-motion`。
