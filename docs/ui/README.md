# LifeTrace UI

LifeTrace 桌面端统一 UI 架构与设计系统文档。任何新增页面、组件或交互改动都必须先阅读本目录。

## 文档索引

| 文档 | 内容 |
| --- | --- |
| [design-system.md](./design-system.md) | Design Tokens：颜色、字体、间距、圆角、阴影、层级 |
| [component-guidelines.md](./component-guidelines.md) | 组件分层与复用规范 |
| [interaction-guidelines.md](./interaction-guidelines.md) | 右键菜单、Toast、加载/空/错误状态、命令面板 |
| [migration-report.md](./migration-report.md) | 本次全量重构的执行记录与验收结果 |

## 快速开始

1. 先读 `design-system.md`，所有 UI 数值必须来自 `src/app/tokens.css` 中的 token。
2. 检查 `src/components/ui`、`src/components/common`、`src/components/layout` 是否已有可用组件，禁止重复造轮子。
3. 页面结构统一为：`AppShell → PageContainer → Toolbar → PageContent`。
4. 开发完成后到 `?view=gallery` 设计画廊确认组件样式在浅色 / 深色主题下均正常。
5. 运行 `npm run lint && npm run test:unit && npm run web:build && npm run browser:build`。

## 关键路径

- 设计 Token：`apps/desktop/app/tokens.css`
- 统一样式层：`apps/desktop/app/hengxu.css`
- UI 基础组件：`apps/desktop/src/components/ui`
- 公共组件：`apps/desktop/src/components/common`
- 布局组件：`apps/desktop/src/components/layout`
- 业务页面：`apps/desktop/src/components/feature`
- 菜单 / 动作系统：`apps/desktop/src/ui`
- 组件画廊：`apps/desktop/src/components/design/DesignGallery.tsx`
