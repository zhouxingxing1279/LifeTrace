# Layout

## 桌面优先

重点尺寸 1280×720 / 1366×768 / 1920×1080 / 2560×1440。窗口缩小时：侧边栏折叠为抽屉、表格横向滚动、工具栏收缩。

## 页面结构

```tsx
<AppShell …>            {/* 由 HengXuShell 提供 */}
  <PageContainer>       {/* 或 hx-view */}
    <Toolbar … />
    <PageContent />
  </PageContainer>
</AppShell>
```

标题由全局 TopBar 显示；页面内不要重复大标题。`PageHeader` 仅在需要额外说明时使用。

## 列表优先

习惯、账户等使用行式列表；账单等数据密集页面使用 Table（金额右对齐、行高舒适、hover、右键菜单）。
