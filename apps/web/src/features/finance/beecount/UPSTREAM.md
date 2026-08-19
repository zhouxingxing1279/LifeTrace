# BeeCount Cloud Web upstream record

## Upstream

- Repository: `TNT-Likely/BeeCount-Cloud`
- Upstream SHA reviewed for this port: `3e02e499431bdceae2051c1dfb980898d26ef5e1`
- License holder: sunxiao / GitHub `TNT-Likely`
- License: BeeCount Cloud Software License Agreement v1.0 (non-commercial use allowed; commercial use requires paid authorization; copyright/license/author notices must be preserved)

## Source paths used as the implementation baseline

The LifeTrace finance workspace was source-reviewed and ported from these upstream areas rather than recreated from screenshots:

- `frontend/apps/web/src/pages/sections/OverviewPage.tsx`
- `frontend/apps/web/src/pages/sections/TransactionsPage.tsx`
- `frontend/apps/web/src/pages/sections/CalendarPage.tsx`
- `frontend/apps/web/src/pages/sections/LedgersPage.tsx`
- `frontend/apps/web/src/pages/sections/BudgetsPage.tsx`
- `frontend/apps/web/src/pages/sections/AccountsPage.tsx`
- `frontend/apps/web/src/pages/sections/CategoriesPage.tsx`
- upstream Tags section and transaction filters in `frontend/apps/web/src/pages/sections`
- `frontend/apps/web/src/App.tsx`
- `frontend/apps/web/src/layout/*`
- `frontend/apps/web/src/context/*`
- `frontend/packages/ui/src/*`
- `frontend/packages/web-features/src/*`
- `frontend/packages/api-client/src/*`

## LifeTrace port strategy

`FinanceWorkspace.tsx` preserves the upstream finance IA and the important behavior boundaries: ledger-scoped overview, transaction filters, finance calendar, ledgers, budgets, accounts, categories, tags and import. It does not reuse the old LifeTrace `BeeCountFinancePage.tsx` visual implementation.

LifeTrace intentionally replaces the following global concerns:

- BeeCount login/authentication → LifeTrace Cloud session
- BeeCount top-level AppShell/sidebar → LifeTrace AppShell
- BeeCount theme/profile shell → LifeTrace semantic Design System
- direct BeeCount API client authentication → `LifeTraceBeeCountAdapter`

The current LifeTrace backend BeeCount integration exposes a read-only aggregate snapshot. Therefore BeeCount-backed pages are read-only where the server contract is read-only. Editable finance CRUD is provided by LifeTrace Native finance entities using the existing Cloud sync contract. No fake BeeCount write capability is introduced.

## Omitted upstream areas

- BeeCount login and registration pages
- admin-only pages
- PWA install/badge/intake code
- BeeCount profile/device settings already owned by LifeTrace
- direct token/local-storage auth state
- upstream global shell and navigation

## Local modifications

1. API calls are translated through `LifeTraceBeeCountAdapter`.
2. Page-level shell is rendered with LifeTrace shadcn-style primitives and semantic tokens.
3. Amount privacy masking follows the LifeTrace global privacy toggle.
4. BeeCount source selection can switch to LifeTrace Native without leaving the finance IA.
5. Import writes only to LifeTrace Native because the BeeCount compatibility endpoint is currently read-only.

## Sync procedure

When updating BeeCount-derived finance UI:

1. Fetch the latest upstream SHA and read changes under the paths above.
2. Compare upstream finance IA, filters, entity cards and analytics behavior with `FinanceWorkspace.tsx` and `beecount/adapter.ts`.
3. Port relevant source changes through the adapter boundary; do not copy upstream auth/AppShell behavior.
4. Update the SHA in this file and record material local deviations.
5. Re-run typecheck, unit tests, production build, finance route smoke tests and responsive/theme checks.
6. Re-check the current upstream license before distribution or commercial use.

## Attribution

BeeCount and BeeCount Cloud remain the work of their original author(s). This port preserves attribution and is not a claim of authorship over the upstream finance client.
