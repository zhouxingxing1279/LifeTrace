# BeeCount Cloud Web upstream record

## Upstream

- Repository: `TNT-Likely/BeeCount-Cloud`
- Upstream SHA pinned for the current port: `3e02e499431bdceae2051c1dfb980898d26ef5e1`
- License holder: sunxiao / GitHub `TNT-Likely`
- License: BeeCount Cloud Software License Agreement v1.0. The bundled license and attribution must remain with the port.

## UI authority

**BeeCount Cloud Web is the UI authority for the LifeTrace finance surface.**

LifeTrace must not independently redesign the finance pages with its generic `Card`, `MetricCard`, `PageHeader` or another LifeTrace-native finance component hierarchy. The only LifeTrace-owned presentation around finance is the outer authenticated AppShell/navigation.

The port tracks these upstream areas directly:

- `frontend/apps/web/src/pages/sections/*`
- `frontend/apps/web/src/components/sections/OverviewSection.tsx`
- `frontend/apps/web/src/components/dashboard/*`
- `frontend/packages/web-features/src/nav.ts`
- `frontend/packages/web-features/src/features/*`
- `frontend/apps/web/src/styles.css`

`beecount-cloud/BeeCountCloudWorkspace.tsx` is the mounted finance workspace. `FinanceWorkspace.tsx` is intentionally only a thin export/mount point. `beecount-cloud/beecount-cloud.css` carries BeeCount Cloud's design tokens in a scoped form so the LifeTrace outer shell is not recolored.

## Platform substitutions

Only platform boundaries are substituted:

- BeeCount standalone login/session -> LifeTrace authenticated session.
- BeeCount global AppShell -> LifeTrace AppShell.
- BeeCount Cloud persistence -> LifeTrace PostgreSQL BeeCount compatibility service.
- BeeCount network client -> `LifeTraceBeeCountAdapter`.
- Route root -> `/app/finance/*`.
- Amount privacy -> LifeTrace global privacy switch.
- BeeCount browser-local active-ledger preference -> React session state. LifeTrace Web deliberately forbids browser-local persistence outside the Vditor draft cache. If active-ledger selection later needs cross-session persistence it must use a server-side `user.preference`, not `localStorage`/IndexedDB.

Everything inside the mounted finance workspace should otherwise follow BeeCount Cloud's information architecture and interaction conventions.

## Data contract rules

BeeCount is the only user-visible finance data source.

- The Web adapter must never fall back to the retired LifeTrace finance model.
- Legacy LifeTrace ledger wire IDs (`lifetrace:*`) are not selectable in the BeeCount Web port.
- The aggregate snapshot endpoint is capped at 500 transactions per request; `snapshotAll()` must consume every page before analytics/list rendering. This is required for ledgers with 500+ transactions.
- Transactions are presented newest-first and the list uses BeeCount Cloud's default page size of 20.
- Accounts/categories/tags are filtered by BeeCount provenance on the backend snapshot boundary.

## Current write boundary

The current LifeTrace Web BeeCount adapter is read-only. Read-only status is an API-boundary limitation, not permission to replace BeeCount UI with a LifeTrace-native implementation.

Until BeeCount Web mutation contracts are exposed by the LifeTrace compatibility backend, write controls that would mutate finance data must remain unavailable or clearly disabled. New Web write capability must be implemented against the BeeCount contract itself; it must never revive the retired LifeTrace finance CRUD/import model.

## Sync procedure

When BeeCount Cloud Web changes:

1. Compare the pinned upstream SHA with the new upstream revision.
2. Review finance changes in the source paths listed above.
3. Port the relevant BeeCount components/interactions into `beecount-cloud/` rather than recreating them with LifeTrace generic UI.
4. Keep LifeTrace changes limited to session, route and API adapters plus documented platform-boundary deviations.
5. Update the pinned SHA in this file and source headers.
6. Run Web typecheck, unit tests and production build; run Cloud/PostgreSQL regressions whenever the compatibility API changes.
7. Re-check the upstream license before distribution or commercial use.

## Attribution

BeeCount and BeeCount Cloud remain the work of their original author(s). This port preserves attribution and does not claim authorship over the upstream finance client.
