# Component Guidelines

## 组件分层

```
src/components/
  ui/        # 基础 UI primitive：Button、Input、Badge、Switch、Tabs、Skeleton…
  common/    # 跨页面复用：EmptyState、ErrorState、LoadingState、StatDisplay、PanelHead…
  layout/    # AppShell、CommandPalette、PageContainer、PageHeader、Toolbar、Section
  feature/   # 业务页面：dashboard / habits / finance / life / settings / forms

src/ui/      # 动作与菜单基础设施：ContextMenu、MoreMenu、confirm、toastBus（保持兼容）
```

## 复用优先级

每新增一个 UI 元素，先按此顺序查找：

1. `src/components/ui`（基础组件）
2. `src/components/common`（公共组件）
3. `src/components/layout`（布局组件）
4. 现有 `feature` 页面内的局部组件
5. 才允许新建

禁止为同一用途写第二个相似实现（例如页面各自实现 Empty State）。

## 页面结构

```tsx
<AppShell …>            {/* 由 HengXuShell 统一提供 */}
  <PageContainer>       {/* 或直接使用 hx-view */}
    <Toolbar … />       {/* 筛选 / 搜索 + 主操作 */}
    <PageContent />     {/* 列表、表格、面板 */}
  </PageContainer>
</AppShell>
```

Page Header 由全局 TopBar 承担，页面内**不要重复大标题**；仅当确有必要时使用 `PageHeader` 增加说明。

## 样式约定

- 新组件样式写入 `apps/desktop/app/hengxu.css`，class 前缀 `lt-`；兼容既有 `hx-` class。
- 颜色、间距、圆角、字体一律用 token。
- 列表优先使用行式布局；数据密集页面使用 Table；独立信息实体才使用 Card。
