# LifeTrace Frontend V2 Capability Inventory

> Branch: `feature/frontend-v2-clean-rewrite`  
> Scope: clean-room rewrite safety inventory before Legacy UI removal  
> Source of truth: `docs/ui/frontend-v2.md`, backend/contracts, current non-visual Desktop capability code and build configuration.

## 1. Rewrite boundary

### Web

`apps/web` is treated as Legacy Frontend Implementation and is removed as one clean-room boundary. The V2 Web application is recreated from backend/contracts/docs/product requirements, not from historical Web JSX/CSS.

### Desktop

Desktop uses **UI clean-room rewrite + native capability preservation**. The renderer/visual layer is replaceable, while platform/domain infrastructure remains intact.

| Path / capability | Decision | Reason |
|---|---|---|
| `apps/desktop/src-tauri/` | KEEP | Tauri runtime, commands, local service and native integration |
| `apps/desktop/src/db/` | KEEP | local persistence boundary |
| `apps/desktop/src/services/` | KEEP | non-visual service/domain behavior |
| `apps/desktop/src/stores/` | KEEP / ADAPT | retain state/domain contracts; visual consumers are rebuilt |
| `apps/desktop/src/types/` | KEEP | domain/platform types |
| `apps/desktop/src/lib/` | KEEP | non-visual shared logic |
| `apps/desktop/src/utils/` | KEEP | non-visual utilities |
| `apps/desktop/package.json` | KEEP / ADAPT | Tauri, TipTap and native dependency inventory must be preserved |
| `apps/desktop/src-tauri/tauri.conf.json` and capabilities | KEEP / ADAPT | packaging/runtime boundary |
| `apps/desktop/app/` | REMOVE | Legacy visual/CSS layer |
| `apps/desktop/src/components/` | REMOVE | Legacy visual component layer; platform contracts are re-exposed through adapters |
| `apps/desktop/src/ui/` | REMOVE | Legacy UI abstraction |
| `apps/desktop/tauri-ui/` | REMOVE + RECREATE | Legacy renderer is replaced by a clean-room V2 entry |
| Legacy page/theme/bootstrap CSS | REMOVE | V2 tokens become the only visual source of truth |

## 2. Desktop native capability matrix

The current Tauri entry registers the following capabilities. They must remain reachable through the V2 Desktop platform adapter or an explicit native workspace.

| Capability | Decision | V2 boundary |
|---|---|---|
| Cloud API transport | KEEP | Desktop platform adapter |
| Cloud authentication transport | KEEP | Desktop platform adapter |
| Credential set/get/clear | KEEP | Desktop secure/auth adapter |
| Client log write/path/recent read | KEEP | Diagnostics / Settings |
| Storage status and migration | KEEP | Native Settings |
| Photo sync status | KEEP | Native tools |
| Mobile upload server start/stop/status | KEEP | Native tools |
| Photo pairing/cancel/recovery | KEEP | Native tools |
| Photo compatibility/certificate export | KEEP | Native tools |
| Note attachment copy/delete/open/show | KEEP | Notes native adapter |
| Native text file read/write | KEEP | Import/export adapter |
| Open external URL | KEEP | Desktop navigation adapter |
| Sync session lifecycle | KEEP | Sync adapter |
| Sync profiles/create/select | KEEP | Sync adapter |
| Sync status/manual sync | KEEP | Shared sync status + Desktop implementation |
| Sync conflict list/resolve | KEEP | Sync conflict UI |
| Encrypted vault initialize/lock/unlock/status | KEEP | Native Vault workspace |
| Vault assets/albums CRUD | KEEP | Native Vault workspace |
| Vault thumbnail/asset read | KEEP | Native Vault workspace |
| Vault trash/restore/permanent delete | KEEP | Native Vault workspace |
| Vault integrity verification | KEEP | Native Vault workspace |
| Vault password/auto-lock/blur-lock | KEEP | Native security settings |
| Tauri process plugin | KEEP | Native platform layer |
| Tauri updater plugin | KEEP | About / Update settings |
| Tauri dialog plugin | KEEP | File/import/export workflows |
| Storage bootstrap + cleanup | KEEP | `src-tauri` startup |
| Sync scheduler | KEEP | `src-tauri` background runtime |
| Photo staging drain | KEEP | `src-tauri` background runtime |
| Local service server | KEEP | `src-tauri` background runtime |

No native capability is removed in the clean-room deletion commit.

## 3. Shared domain inventory

Generated contracts expose the domain facts used by V2 without relying on Legacy UI, including:

- Activities and activity logs for habits/fitness tracking;
- daily review data;
- English articles, highlights, notes, vocabulary and reading records;
- Finance accounts and money values expressed in minor currency units;
- Notes, folders, tags, revisions and entity relations;
- Files and storage metadata;
- authentication/device/session contracts;
- sync push/pull/snapshot/conflict contracts.

These contracts, plus backend/domain docs, are the functional source for shared feature models.

## 4. V2 shared architecture decision

The rewrite uses one shared React feature/rendering layer owned by the V2 Web source tree and consumed by both shells:

```text
apps/web/src/v2/
  design-system/
  features/
  platform/
  App.tsx

apps/web/             Web entry + Web adapter
apps/desktop/tauri-ui/  clean-room Desktop entry
apps/desktop/src/platform-v2/ Desktop native adapter
```

This keeps the initial migration compatible with the existing monorepo and Tauri build while preventing a second Desktop copy of feature UI. A later package extraction is allowed when the workspace is converted to formal package workspaces; shared ownership is required now regardless of physical directory name.

## 5. Execution gates

The deletion step may proceed only with these invariants:

- `main` remains the recovery baseline;
- target branch remains `feature/frontend-v2-clean-rewrite`;
- `AGENTS.md` clean-room rules are present;
- `src-tauri` and non-visual Desktop layers are not deleted;
- Web Legacy implementation is not consulted after deletion;
- old Desktop visual implementation is not consulted after deletion;
- new tokens/primitives/shell are implemented before feature styling;
- CI guards are updated to validate V2 architecture rather than historical DOM/file names.
