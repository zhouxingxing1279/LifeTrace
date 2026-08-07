# Migration Report

本次对 LifeTrace 桌面端前端执行了全量 UI/UX 重构，不改变数据模型、IPC、云同步与业务逻辑。

## 1. 重构内容

- 建立完整 Design Token（颜色 / 字体 / 间距 / 圆角 / 阴影 / 层级 / 动效 / 布局），支持浅色与深色主题。
- 合并三套互相覆盖的样式层（`hengxu.css`、`redesign.css`、`ui-foundation.css`）为一套统一的 `hengxu.css`，删除两套旧层。
- 建立 UI primitives（Button、Input、Select、Textarea、Field、Badge、Switch、Checkbox、Tabs、Skeleton、Spinner、Kbd、Tooltip、SearchInput）。
- 建立公共组件（EmptyState、ErrorState、LoadingState、StatDisplay、PanelHead、MobileUploadControl）。
- 建立布局组件（AppShell、Sidebar、TopBar、CommandPalette、PageContainer、PageHeader、Toolbar、Section）。
- 新增全局命令面板（Ctrl+K / Ctrl+Shift+P）。
- 拆分 583 行单体 `HengXuShell.tsx` 为 feature 模块页面。
- 完成 14 个页面 / 模块的迁移与统一（总览、AI、坚持、英语、健身、照片、笔记、财务、账单、账户、导入、日历、复盘、设置）。
- 保留并强化右键菜单（习惯、账单、账户），危险操作二次确认。
- 建立组件演示画廊（`?view=gallery`，Storybook 等效方案）。
- 增加 UI 基础设施单元测试。

## 2. 页面迁移情况

| 页面 | 状态 |
| --- | --- |
| 总览 Dashboard | ✅ |
| AI 管家 | ✅ |
| 坚持 Habits | ✅ |
| 每日英语 | ✅ |
| 健身 Fitness | ✅ |
| 照片 Photos | ✅ |
| 笔记 Notes | ✅ |
| 财务概览 | ✅ |
| 账单管理 | ✅ |
| 账户管理 | ✅ |
| 账单导入 | ✅ |
| 生活日历 | ✅ |
| 每日复盘 | ✅ |
| 设置 | ✅ |
| 成长树 Growth | ⏸ 仅存在于 web-client（`web-client/src/pages/GrowthPages.tsx`），桌面端未启用，未改动 |

## 3. 删除内容

- `apps/desktop/app/redesign.css`（旧 2026 界面层，含大圆角 / 渐变 / 衬线标题）
- `apps/desktop/app/ui-foundation.css`（已被统一层取代）
- `HengXuShell.tsx` 内联的 18 个页面函数（迁至 `src/components/feature`）
- `auth-shell-fixes.css` 中隐藏顶栏操作区的残留规则

## 4. 测试结果

| 项目 | 结果 |
| --- | --- |
| `npm run lint`（tsc --noEmit） | ✅ |
| `npm run test:unit`（93 项） | ✅ |
| `npm run web:build`（tauri 前端） | ✅ |
| `npm run browser:build`（web-client） | ✅ |
| 应用启动 + 本地服务健康检查 | ✅ |
| 14 页面浅色 / 深色渲染（无头浏览器快照） | ✅ |

## 5. 尚未解决的问题

- 成长树仅存在于 web-client，桌面端没有入口，本次未迁移。
- 照片 / 加密相册与英语模块保留了各自的模块样式，仅做了变量桥接与深色主题表面覆盖；如需完全统一需进一步收敛。
- Storybook 未引入（Vite 8 + React 19 的版本兼容成本高于收益），以应用内设计画廊 + 单元测试等效覆盖。
- 无头浏览器无法验证右键菜单点击流与命令面板键盘交互，建议在真实 Tauri 窗口中人工过一遍。

## 6. 后续建议

1. 用真实窗口回归右键菜单、命令面板与模态框焦点行为。
2. 收敛照片 / 英语模块样式到统一 token，移除残留硬编码色值。
3. 为记账表格接入虚拟滚动（数据量增大后）。
4. 将 `web-client` 与桌面端共享同一套 `ui` primitives，避免双份组件。
5. 引入视觉回归测试（截图基线）保护设计系统。
