# Interaction Guidelines

## 右键菜单（Context Menu）

桌面应用的核心交互。列表项（习惯、账单、账户、照片、笔记）必须提供右键菜单与“更多”按钮：

- 菜单项按 `primary → related → organize → danger` 分组。
- 危险操作（删除、清空）必须走 `confirmAction()` 二次确认。
- 键盘可达：菜单按钮可 focus，`Enter` 触发，`Esc` 关闭。

## Toast

- 使用 `notify(message, type)`（`src/ui/feedback/toastBus.ts`）。
- 自动消失：普通 2500ms，错误 4500ms。
- 禁止用永久页面提示代替 Toast；复杂解释用 Tooltip 或 Help Dialog。

## 加载 / 空 / 错误

- 加载：`LoadingState` / `Skeleton`，禁止满屏转圈。
- 空数据：`EmptyState`（标题 + 一句提示 + 可选操作），禁止长篇引导文案。
- 错误：`ErrorState`（明确操作失败 + 简短原因 + 重试），详细错误进入日志，不向普通 UI 暴露堆栈。

## 命令面板

- `Ctrl+K` / `Ctrl+Shift+P` 打开全局命令面板。
- 支持页面跳转、新建习惯 / 记账 / 账户、切换主题与密度。

## 无障碍

- 按钮可键盘 focus，焦点环使用 `--ui-focus`。
- Dialog 焦点陷阱、`aria-modal`、`aria-label` 齐全。
- 输入控件必须有 label；颜色对比满足可读性。
