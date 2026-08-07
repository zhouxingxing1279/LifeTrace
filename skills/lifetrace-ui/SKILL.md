---
name: lifetrace-ui
description: LifeTrace 前端 UI 开发规范。当任务涉及新增页面、修改页面、新增 UI 组件、新增交互或新增业务模块时使用，确保界面遵循统一设计系统、组件复用与桌面软件交互规范。
---

# LifeTrace UI Skill

LifeTrace 是桌面生产力软件（Tauri + React 19 + Vite），不是 SaaS Landing Page。任何 UI 改动都必须保持统一、成熟、简洁的桌面软件质感。

## 何时使用

- 新增页面或业务模块
- 修改现有页面
- 新增 UI 组件或交互
- 调整布局、主题或样式

## 必须执行的流程

1. **阅读设计系统**：先读 `references/design-system.md` 与 `apps/desktop/app/tokens.css`。
2. **检查已有组件**：按 `src/components/ui` → `src/components/common` → `src/components/layout` → `src/components/feature` 顺序查找可复用组件。
3. **优先复用**：禁止为同一用途重复实现（EmptyState / LoadingState / ErrorState / Button / Badge / Dialog / ContextMenu 等）。
4. **遵循 token**：颜色、间距、圆角、字号一律使用 `--ui-*` token，禁止硬编码 UI 数值。
5. **遵循页面布局**：`AppShell → PageContainer → Toolbar → PageContent`；页面标题由 TopBar 承担，不要重复大标题。
6. **遵循交互规范**：列表项提供右键菜单与“更多”按钮；危险操作走 `confirmAction` 二次确认；提示用 `notify()` Toast，自动消失。
7. **反馈三件套**：加载用 Skeleton / LoadingState；空数据用 EmptyState；错误用 ErrorState（含重试），详细错误进日志。
8. **主题**：浅色与深色主题都必须验证（`?theme=dark`）。
9. **禁止 AI Dashboard 风格**：见 `references/anti-patterns.md`。
10. **验证**：`npm run lint`、`npm run test:unit`、`npm run web:build`、`npm run browser:build`；新组件到 `?view=gallery` 画廊确认。

## 参考文档

- [design-system.md](./references/design-system.md) — Token 与视觉规则
- [layout.md](./references/layout.md) — 布局与页面结构
- [components.md](./references/components.md) — 组件目录与复用
- [interaction.md](./references/interaction.md) — 右键菜单 / Toast / 反馈
- [anti-patterns.md](./references/anti-patterns.md) — 禁止事项

## 验收标准

每个改动完成必须满足：功能正常、无 TS/lint 错误、使用统一 token、复用公共组件、深浅主题正常、Loading/Empty/Error 正常、无 console error、无重复组件。
