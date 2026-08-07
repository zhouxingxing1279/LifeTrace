# Interaction

## 右键菜单

列表项必须提供 `ContextMenu`（右键）与 `MoreMenu`（更多按钮）。菜单分组：primary → related → organize → danger。危险操作必须 `confirmAction()` 确认。

## Toast

`notify(message, type)`：普通 2500ms，错误 4500ms，自动消失。禁止永久页面提示。

## 反馈三件套

- 加载：`LoadingState` / Skeleton。
- 空数据：`EmptyState`，标题 + 一句提示 + 可选操作。
- 错误：`ErrorState`，明确失败操作 + 简短原因 + 重试；详细错误进日志。

## 命令面板

Ctrl+K / Ctrl+Shift+P；新增页面或常用操作时同步加入命令项（`HengXuShell.tsx` 中 `commandItems`）。

## 无障碍

按钮可键盘 focus、Dialog 焦点陷阱、aria-label、输入 label、颜色对比。
