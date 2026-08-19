# Web redesign implementation record

This file tracks execution of `docs/ui/web-redesign.md`. It is not a replacement for the authoritative redesign document.

## Branch

- implementation branch: `agent/web-redesign`
- new application root: `apps/web`
- legacy UI remains in `apps/desktop/web-client` until cutover gates are green

## Route / reference / data matrix

| Route | Design reference | Data contract | Status |
|---|---|---|---|
| `/login` | Preline login + shadcn form + Catalyst typography | `AuthApi` | Implemented |
| `/app/today` | Shadcnblocks dashboard + Tremor + Catalyst | Cloud entities | Implemented |
| `/app/execution` | Shadcnblocks todo + Catalyst list/detail | execution entities | Implemented |
| `/app/calendar` | Shadcnblocks calendar + Catalyst toolbar | execution task/calendar entities | Implemented |
| `/app/habits` | Tremor + Shadcnblocks | habit entities | Implemented |
| `/app/fitness` | Tremor analytics | workout entities | Implemented |
| `/app/health` | trend-first analytics | currently available workout/activity contract | Implemented; health-provider metrics blocked by backend schema |
| `/app/notes` | Catalyst workspace + Preline | note entities | Implemented |
| `/app/english/*` | Catalyst content + shadcn | english entities | Implemented |
| `/app/review` | Shadcnblocks + Tremor | review entities | Implemented |
| `/app/finance/*` | BeeCount Cloud Web source + LifeTrace AppShell | BeeCount adapter + native finance entities | Implemented |
| `/app/assistant` | shadcn conversational layout | existing Assistant API | Implemented |
| `/app/search` | shadcn Command | `searchEntities` | Implemented |
| `/app/settings/*` | Catalyst settings + shadcn form | session/preferences | Implemented |

## Architecture decisions

- `apps/web` is a standalone React/Vite application and does not import legacy Web UI files.
- Existing Cloud sync/auth/entity contracts were moved into the new application service layer so backend behavior is not rewritten as part of the UI project.
- Global shell is responsive: desktop sidebar, mobile bottom navigation, More sheet, and Ctrl/Cmd+K command palette.
- Theme is semantic-token based and has light/dark/system modes. Theme preference uses a cookie for first paint and the existing `user.preference` entity for account sync when available.
- Business entities are never stored in localStorage or IndexedDB.
- Health metrics not present in the current Cloud schema are shown as unavailable rather than fabricated.

## BeeCount source reuse

See `apps/web/src/features/finance/beecount/UPSTREAM.md`. The upstream SHA, imported/reviewed paths, local modifications, omissions, license boundary and sync procedure are recorded there.

The LifeTrace compatibility API currently marks BeeCount integration data `readOnly: true`; therefore BeeCount views are read-only where the backend is read-only. LifeTrace Native finance provides editable transactions, accounts, categories, budgets, and CSV import under the same finance information architecture.

## Verification gates

The branch adds `.github/workflows/web-v2.yml` with these required automated gates:

1. TypeScript typecheck
2. Vitest unit tests
3. Vite production build
4. production preview startup
5. direct-route smoke requests for all core and finance routes
6. architecture guard preventing legacy UI imports and browser business-state persistence
7. BeeCount upstream attribution/SHA guard

Cutover of root scripts, Docker/Caddy build input, legacy workflow replacement, and deletion of `apps/desktop/web-client` must happen only after these gates are green.
