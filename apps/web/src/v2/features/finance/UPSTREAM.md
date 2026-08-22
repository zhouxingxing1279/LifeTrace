# Finance V2 upstream boundary

LifeTrace Frontend V2 is a clean-room rewrite, with one explicit external reuse exception: Finance may reuse the mature BeeCount Web interaction model.

## Pinned upstream

- Repository: `TNT-Likely/BeeCount-Cloud`
- Commit: `3e02e499431bdceae2051c1dfb980898d26ef5e1`
- License reported by the upstream repository: AGPL-3.0
- Upstream Web areas reviewed for the Finance V2 information architecture:
  - `apps/web/src/pages/sections/OverviewPage.tsx`
  - `TransactionsPage.tsx`
  - `CalendarPage.tsx`
  - `LedgersPage.tsx`
  - `BudgetsPage.tsx`
  - `AccountsPage.tsx`
  - `CategoriesPage.tsx`
  - `TagsPage.tsx`
  - `ImportPage.tsx`
  - `packages/web-features/src/components/TransactionList.tsx`
  - `packages/web-features/src/components/TransactionRow.tsx`
  - `packages/web-features/src/features/TransactionsPanel.tsx`
  - `packages/web-features/src/features/AccountsPanel.tsx`
  - `packages/web-features/src/features/BudgetsPanel.tsx`
  - `packages/web-features/src/features/CategoriesPanel.tsx`
  - `packages/web-features/src/features/LedgerOverviewPanel.tsx`

## LifeTrace adaptation rule

BeeCount is the Finance business-interaction authority, not a second visual system. LifeTrace V2 preserves the mature finance concepts and route hierarchy while rendering them only through LifeTrace V2 tokens, primitives, shell, accessibility rules, responsive rules and platform adapters.

The shared Finance route hierarchy is:

- `/app/finance`
- `/app/finance/transactions`
- `/app/finance/calendar`
- `/app/finance/ledgers`
- `/app/finance/budgets`
- `/app/finance/accounts`
- `/app/finance/categories`
- `/app/finance/tags`
- `/app/finance/import`

Web and Desktop consume the same Finance implementation. Desktop does not fork BeeCount-derived UI.

## Licensing and attribution

Do not silently copy upstream source files. Any direct source reuse must preserve the upstream license and required notices. The current V2 implementation adapts the verified interaction architecture and business concepts into the LifeTrace shared Design System rather than importing BeeCount's visual layer.
