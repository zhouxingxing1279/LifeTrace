# Third-Party Notices

LifeTrace incorporates or derives selected user-interface concepts and source-level behavior from third-party projects. This file records material attribution required by the Web application.

## BeeCount Cloud

- Project: BeeCount Cloud
- Upstream repository: `TNT-Likely/BeeCount-Cloud`
- Author / copyright holder: sunxiao (GitHub: `TNT-Likely`)
- Upstream revision reviewed for the LifeTrace Web finance port: `3e02e499431bdceae2051c1dfb980898d26ef5e1`
- License: BeeCount Cloud Software License Agreement v1.0
- LifeTrace port record: `apps/web/src/features/finance/beecount/UPSTREAM.md`
- Preserved notice: `apps/web/src/features/finance/beecount/LICENSE.BeeCount-Cloud.txt`

The LifeTrace finance workspace was implemented from source review of BeeCount Cloud Web, including its Overview, Transactions, Calendar, Ledgers, Budgets, Accounts, Categories, Tags, and Import information architecture. LifeTrace does not claim authorship of BeeCount or BeeCount Cloud.

The upstream license permits specified non-commercial uses subject to its terms and requires preservation of copyright, license, and author information. Commercial use requires separate authorization from the upstream copyright holder. The authoritative license is the current `LICENSE` file in the upstream repository; review it before redistribution or commercial deployment.

## Vditor

- Project: Vditor
- Upstream repository: `Vanessa219/vditor`
- Package integrated by LifeTrace Web: `vditor@3.11.3`
- Copyright: 2019-present B3log 开源, b3log.org
- License: MIT
- LifeTrace integration record: `apps/web/src/features/notes/vditor/UPSTREAM.md`
- Preserved license: `apps/web/src/features/notes/vditor/LICENSE.Vditor.txt`

LifeTrace uses Vditor directly as the browser Markdown editor for the Notes module. Vditor provides the Markdown editing/rendering engine and editing modes; LifeTrace owns the surrounding note list, authentication, Cloud persistence, autosave, search and application navigation.

## UI and runtime dependencies

The Web client also uses third-party npm dependencies declared in `apps/web/package.json`, including React, React Router, Vite, Tailwind CSS, Radix UI packages, Lucide, Recharts, Zustand, Vitest, Playwright and Vditor. Their respective package licenses remain applicable. No ownership claim is made over those projects.
