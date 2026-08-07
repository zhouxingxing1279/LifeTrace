# Components

## 目录

```
src/components/ui/       基础 primitive（Button、Input、Badge、Switch、Tabs、Skeleton…）
src/components/common/   公共组件（EmptyState、ErrorState、LoadingState、StatDisplay…）
src/components/layout/   AppShell、CommandPalette、PageContainer、PageHeader、Toolbar、Section
src/components/feature/  业务页面
src/ui/                  菜单 / 动作基础设施（ContextMenu、MoreMenu、confirm、toastBus）
```

## 复用顺序

`ui` → `common` → `layout` → feature 内部 → 新建。禁止重复实现。

## 新增组件

- class 前缀 `lt-`，样式写入 `hengxu.css`，全部使用 token。
- 新 primitive 同步到 `?view=gallery` 画廊与 `docs/ui/component-guidelines.md`。
