# Vditor upstream record

## Upstream

- Project: Vditor
- Repository: `Vanessa219/vditor`
- Package: `vditor@3.11.3`
- License: MIT
- Copyright: 2019-present B3log 开源, b3log.org

## LifeTrace integration

LifeTrace uses Vditor directly as the browser note body editor. The surrounding note list, LifeTrace Cloud persistence, search, delete behavior, mobile list/editor navigation and autosave status remain LifeTrace application concerns.

The Vditor integration intentionally configures:

- `mode: "ir"` by default for Typora-like instant rendering.
- Vditor's built-in edit-mode switch so users can move between instant rendering, WYSIWYG and split-view editing.
- GFM/CommonMark-oriented features including tables, task lists, fenced code, links, footnotes and automatic links.
- code highlighting, math rendering, outline, preview and fullscreen tools.
- note-scoped Vditor `localStorage` cache. The cache is a crash/offline draft safety layer only; LifeTrace Cloud remains the authoritative persistence and synchronization destination.
- a user + note scoped cache key to avoid collisions across notes or accounts on the same browser.
- recovery reconciliation: a locally cached dirty draft is restored into React state and goes through the normal Cloud autosave path; a clean/stale cache never overrides a newer Cloud value.
- successful Cloud saves mark the local draft clean, so future Cloud revisions can safely win over stale cached content.
- self-hosted Vditor runtime assets copied from the pinned npm package into the Web public bundle during `postinstall`, so the editor does not depend on a third-party CDN at runtime.
- LifeTrace light/dark theme synchronization through Vditor `setTheme`.

Upload/record controls are deliberately omitted until they can be wired to LifeTrace file storage. They must not be enabled with a browser-local or unrelated upload backend.

## Updating

When upgrading Vditor, update the npm version, re-check the upstream MIT license, ensure `scripts/copy-vditor-assets.mjs` still matches the package layout, then run Web typecheck, unit tests, production build and browser tests.
