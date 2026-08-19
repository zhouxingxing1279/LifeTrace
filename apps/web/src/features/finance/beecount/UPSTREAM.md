# BeeCount Cloud Web upstream record

## Upstream

- Repository: `TNT-Likely/BeeCount-Cloud`
- Upstream SHA reviewed for this port: `3e02e499431bdceae2051c1dfb980898d26ef5e1`
- License holder: sunxiao / GitHub `TNT-Likely`
- License: BeeCount Cloud Software License Agreement v1.0. The bundled license and attribution must remain with the port.

## Source paths used as the implementation baseline

The LifeTrace finance workspace is source-derived from BeeCount Cloud Web rather than recreated from screenshots. The reviewed areas include `frontend/apps/web/src/pages/sections/*`, `frontend/apps/web/src/App.tsx`, `frontend/apps/web/src/layout/*`, `frontend/apps/web/src/context/*`, `frontend/packages/ui/src/*`, `frontend/packages/web-features/src/*` and `frontend/packages/api-client/src/*`.

## BeeCount-only finance architecture

As of the 2026-08 cutover, BeeCount is the only runtime finance implementation/data source for LifeTrace Web.

- `FinanceWorkspace.tsx` owns Overview, Transactions, Calendar, Ledgers, Budgets, Accounts, Categories, Tags and Import using BeeCount data only.
- `/app/finance/transactions` delegates to the same workspace instead of maintaining a LifeTrace-native transaction implementation.
- Finance pages do not fall back to `finance.*` browser entities when BeeCount is unavailable. Failure is surfaced as a BeeCount availability error.
- Database-backed Cloud deployments do not mount the legacy LifeTrace finance CRUD routes; the production backend finance surface is BeeCount. Those old routes are retained only in the no-database in-memory protocol test harness so historical repository contract tests remain runnable.
- LifeTrace continues to own the global web session, AppShell, design system and privacy masking. Those are platform concerns, not an alternate finance store.
- Existing canonical `finance.*` storage records may still exist internally because the BeeCount compatibility layer maps BeeCount entities into the common PostgreSQL sync log. They are an implementation detail of the BeeCount compatibility service, not a second Web finance data source or user-selectable finance mode.

The current aggregate adapter remains read-only for Web mutations. This cutover deliberately removes the previous behavior where writes were redirected to a separate LifeTrace-native finance model. Write capability must be added to the BeeCount contract itself; it must never reintroduce a native fallback.

## LifeTrace replacements around the BeeCount port

- BeeCount login shell → LifeTrace authenticated AppShell/session
- BeeCount global navigation → LifeTrace AppShell
- BeeCount global theme/profile shell → LifeTrace semantic design system
- BeeCount API access → `LifeTraceBeeCountAdapter`
- Amount privacy → LifeTrace global privacy toggle

## Omitted upstream areas

- BeeCount standalone login and registration pages
- admin-only pages
- PWA install/badge intake code
- duplicate profile/device settings already owned by LifeTrace
- direct token/local-storage auth state
- upstream global shell and navigation

## Sync procedure

When updating BeeCount-derived finance UI:

1. Review the latest upstream SHA and the finance section changes.
2. Compare BeeCount finance IA, filters, entity cards and analytics with `FinanceWorkspace.tsx` and `beecount/adapter.ts`.
3. Port relevant changes without introducing any `LifeTrace Native` finance fallback.
4. Update the upstream SHA and material deviations here.
5. Re-run Web typecheck, unit tests, production build, finance direct-route smoke tests and architecture guards.
6. Re-check the upstream license before distribution or commercial use.

## Attribution

BeeCount and BeeCount Cloud remain the work of their original author(s). This port preserves attribution and does not claim authorship over the upstream finance client.
