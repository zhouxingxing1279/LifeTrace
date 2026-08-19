# Web redesign implementation record

This file tracks execution of `docs/ui/web-redesign.md`. It is not a replacement for the authoritative redesign document.

## Branch and cutover

- implementation branch: `agent/web-redesign`
- pull request: `#97`
- new application root: `apps/web`
- root `web:*` / `browser:*` scripts: switched to `apps/web`
- production `deploy/cloud/Dockerfile.web`: switched to `apps/web/dist`
- Browser Web and Web Container Image workflows: switched to `apps/web`
- legacy UI: retained only until all final cutover gates are green; then `apps/desktop/web-client` is deleted in the final cutover commit

## Route / reference / data matrix

| Route | Design reference | Data contract | Status |
|---|---|---|---|
| `/login` | Preline login + shadcn form + Catalyst typography | `AuthApi` | Implemented |
| `/app/today` | Shadcnblocks dashboard + Tremor + Catalyst | Cloud entities | Implemented |
| `/app/execution` | Shadcnblocks todo + Catalyst list/detail | execution task/project entities | Implemented: Inbox / Today / Upcoming / Projects / Completed |
| `/app/calendar` | Shadcnblocks calendar + Catalyst toolbar | execution task/calendar entities | Implemented: Month / Week / Day / Agenda; mobile default Agenda |
| `/app/habits` | Tremor + Shadcnblocks | habit entities | Implemented: today check-in, streak, 7/30-day rate, 30-day heatmap |
| `/app/fitness` | Tremor analytics | workout entities | Implemented |
| `/app/health` | trend-first analytics | currently available workout/activity contract | Implemented; provider-only health metrics are `BLOCKED_BY_BACKEND` |
| `/app/notes` | Catalyst workspace + Preline | note entities | Implemented |
| `/app/english/*` | Catalyst content + shadcn | english article/highlight/note/vocabulary/learning entities | Implemented: reader, visual highlight, quick note, read history |
| `/app/review` | Shadcnblocks + Tremor | review entities | Implemented |
| `/app/finance` and finance subroutes | BeeCount Cloud Web source + LifeTrace AppShell | BeeCount adapter + native finance entities | Implemented |
| `/app/finance/transactions` | BeeCount Transactions IA + LifeTrace Native editor | finance transaction entities / BeeCount read-only snapshot | Implemented: add/edit/delete/filter |
| `/app/assistant` | shadcn conversational layout | existing Assistant API | Implemented |
| `/app/search` | shadcn Command | `searchEntities` | Implemented |
| `/app/settings/*` | Catalyst settings + shadcn form | session/preferences/management APIs | Implemented: Profile / Appearance / Cloud & Sync / Devices / Privacy / Security / Data / About / Danger Zone |
| `/app/system/ui` | shadcn-style primitives / Tremor visual baseline | none | Implemented UI Showcase |

## Architecture decisions

- `apps/web` is a standalone React/Vite application and does not import legacy Web UI files.
- Existing Cloud sync/auth/entity contracts were copied into the new application service layer so backend behavior is not rewritten as part of the UI project.
- Feature routes use `React.lazy` + `Suspense`; loading fallbacks use semantic Skeleton surfaces.
- Global shell is responsive: desktop collapsible sidebar, mobile bottom navigation, More sheet, and Ctrl/Cmd+K command palette.
- Theme is semantic-token based and has light/dark/system modes. Theme preference uses a cookie for first paint and the existing `user.preference` entity for account sync when available.
- Business entities are never stored in localStorage, sessionStorage, IndexedDB, or another browser-side business database.
- Global uncaught errors are captured by an application Error Boundary in addition to client observability handlers.
- Dialog and Sheet primitives trap keyboard focus, support Escape close, and restore the previously focused element.
- Health metrics not present in the current Cloud schema are shown as unavailable rather than fabricated.

## Design system baseline

The reusable primitive layer under `apps/web/src/components/ui` covers the minimum redesign set: Button, Input, Textarea, Select, Checkbox, Switch, Tabs, Card, Badge, Table, Dialog, AlertDialog, Sheet/Drawer, DropdownMenu, Popover, Tooltip, Command, Skeleton, Separator, ScrollArea, Toast, Progress, EmptyState, PageHeader, MetricCard and Section.

`/app/system/ui` is the runnable visual showcase used to inspect form states, buttons, metric cards, charts, tables, overlays, loading surfaces, feedback and focus behavior under the same semantic tokens used by production pages.

## BeeCount source reuse

See `apps/web/src/features/finance/beecount/UPSTREAM.md`. The upstream SHA, reviewed source paths, local modifications, omissions, license boundary and sync procedure are recorded there. Repository-level attribution is also recorded in `THIRD_PARTY_NOTICES.md`.

The LifeTrace compatibility API currently marks BeeCount integration data `readOnly: true`; therefore BeeCount views are read-only where the backend is read-only. LifeTrace Native finance provides editable transactions, accounts, categories, budgets, and CSV import under the same finance information architecture.

### Explicit backend-blocked BeeCount capabilities

The following are intentionally **not faked in the Web client** and are recorded as `BLOCKED_BY_BACKEND` until LifeTrace exposes compatible write/realtime contracts:

- `BLOCKED_BY_BACKEND`: BeeCount transaction/account/category/budget writes through the BeeCount compatibility adapter; current compatibility response is read-only.
- `BLOCKED_BY_BACKEND`: shared-ledger member management and settlement writes when the compatibility API does not expose those commands.
- `BLOCKED_BY_BACKEND`: BeeCount realtime/WebSocket synchronization when the LifeTrace compatibility API exposes only request/response snapshots.

These backend limitations do not block LifeTrace Native finance CRUD; `/app/finance/transactions` supports create, edit, delete and filter through the existing LifeTrace Cloud sync contract.

## Verification gates

`.github/workflows/web-v2.yml` enforces the new-client gates:

1. TypeScript typecheck
2. Vitest unit tests
3. Vite production build
4. production preview startup
5. direct-route smoke requests for all core, finance and UI-showcase routes
6. Playwright browser matrix at 360 / 390 / 430 / 768 / 1024 / 1366 / 1440 / 1920 widths
7. Light / Dark / System theme checks and reduced-motion media behavior
8. direct-route refresh, Back / Forward and Ctrl/Cmd+K keyboard navigation
9. auth-expiration redirect, loading, empty and API-error states
10. Calendar Month / Week / Day / Agenda checks
11. English reader highlight / quick-note / read-state checks
12. Dialog focus behavior and keyboard accessibility checks
13. architecture guard preventing legacy UI imports and browser business-state persistence
14. BeeCount upstream attribution/SHA guard

The cutover workflows add two additional production gates:

- `.github/workflows/browser-web.yml`: new Web typecheck/unit/build plus existing Cloud auth/assistant Rust regression and clippy checks.
- `.github/workflows/web-image.yml`: production Caddy validation, Compose validation and Docker image build using `apps/web/dist`.

The legacy directory is removed only after the latest-head versions of these gates are green. The PR is merged to `main` only after the post-deletion gates are also green.
