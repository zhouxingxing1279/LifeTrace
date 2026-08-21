# LifeTrace Frontend V2 Agent Rules

Frontend V2 is a clean-room rewrite.

## Scope and source-of-truth rules

- Do not inspect, restore, copy, or derive implementation from historical `apps/web` UI code.
- Do not inspect, restore, copy, or derive visual implementation from historical Desktop UI / renderer code.
- Do not use Git history as a frontend implementation reference.
- Derive behavior from backend APIs, contracts, docs, tests, native capability and explicit product requirements.
- Desktop native/domain contracts may be inspected only to preserve capability.
- Treat Legacy Web and Desktop visual implementation as nonexistent after the clean-room removal commit.

## Shared frontend architecture

- Web and Desktop must share the V2 Design System and feature architecture.
- Follow `docs/ui/apple-design-system.md` as the mandatory visual and interaction baseline.
- Shared design tokens are the only visual source of truth.
- Do not invent page-local colors, radius, spacing, blur, shadow, or motion systems.
- New primitives belong in the shared Design System, not inside one feature.
- Platform differences belong behind Web/Desktop adapters.

## Visual rules

- Content first.
- Liquid Glass is limited to navigation, toolbars, floating controls and transient overlays.
- Do not use Liquid Glass for content cards, tables, lists, editors, statistics blocks or page backgrounds.
- Avoid admin-template card walls, decorative gradients, heavy shadows and page-specific design languages.
- Support light/dark themes, focus-visible states, reduced motion, keyboard navigation and responsive/window-aware layouts.

## Finance

- Finance may reuse BeeCount Web implementation as the business interaction authority.
- BeeCount-derived Finance UI must adopt LifeTrace V2 tokens, primitives and shell.
- Web and Desktop must share one Finance feature implementation rather than diverging copies.

## Completion

- Do not stop at scaffolding.
- Continue through functional regression, TypeScript checks, unit tests, Web E2E, Desktop smoke tests, Rust tests, Web build and Tauri build where CI/runtime support exists.
- Keep documentation synchronized with the implementation.
