# LifeTrace Frontend V2 — Unified Rewrite Plan

> 状态：V2 / 主入口文档  
> 分支：`feature/frontend-v2-clean-rewrite`

本文件是 LifeTrace V2 前端重构的总入口。

详细规范：

- Web：`docs/ui/web-redesign.md`
- Desktop：`docs/ui/desktop-redesign.md`

---

## 1. 总目标

LifeTrace V2 不再把 Web 和 Desktop 当成两个独立 UI 项目。

目标架构：

```text
                    LifeTrace Frontend V2
                              │
            ┌─────────────────┴─────────────────┐
            │                                   │
     Shared Design System                Shared Feature Layer
            │                                   │
            └─────────────────┬─────────────────┘
                              │
             ┌────────────────┴────────────────┐
             │                                 │
           Web                              Desktop
     responsive shell                  Tauri desktop shell
      cloud adapters                  native/local adapters
```

原则：

> 同一个产品、同一套设计语言、同一套 feature，不同平台做合理增强。

---

## 2. Clean-room 策略

### Web

`apps/web` 现有实现视为 Legacy Frontend，允许整体删除后从零重建。

### Desktop

Desktop 只 clean-room 删除 UI / renderer，不删除 Tauri、Local DB、文件系统、同步等平台能力。

Desktop 必须先做 capability inventory 和 boundary extraction，再移除旧 UI。

---

## 3. V2 唯一事实来源

新前端的产品功能从以下来源推导：

```text
services/
contracts/
crates/
docs/
LifeTrace_Future_Requirements.md
后端 API
数据库 / domain model
业务测试
Tauri native capability
明确产品需求
BeeCount Web（仅 Finance）
```

Legacy Web / Desktop UI 不是视觉或组件实现依据。

---

## 4. 共享层

V2 应优先形成以下共享抽象：

```text
Design Tokens
Typography
Primitive Components
Navigation patterns
Data display
Feedback states
Domain schemas
Feature components
Feature view models
Business hooks
Validation
Formatting
```

平台差异必须集中在 adapter 层。

---

## 5. 平台职责

### Web

重点：

```text
responsive
mobile/tablet/desktop
browser navigation
PWA / browser capability
cloud-first
```

### Desktop

重点：

```text
keyboard-first
window-aware
multi-pane
local-first capability
filesystem
native dialog
import/export
Tauri invoke
updater
native productivity interactions
```

Desktop 不得退化成“Web 页面套壳”。

---

## 6. 统一设计方向

参考体系继续使用：

```text
shadcn/ui
Tailwind Plus / Catalyst
Shadcnblocks
Tremor
Preline UI
TailAdmin
Magic UI
Aceternity UI
```

总体气质：

```text
Linear
Raycast
Notion / Notion Calendar
Vercel
成熟桌面 productivity software
```

禁止：

```text
Admin Template 感
大面积渐变
Glassmorphism 全站化
KPI Card Wall
页面级独立设计语言
旧 CSS override chain
为了兼容 Legacy DOM 妥协 V2
```

---

## 7. Finance 规则

Finance 是唯一允许显式复用外部成熟前端实现的业务模块。

策略：

```text
BeeCount Web
    ↓
抽取 Finance Feature
    ↓
适配 LifeTrace V2 Token / Shell
    ↓
Web + Desktop 共用
```

禁止 Web、Desktop、BeeCount 三套财务 UI 并存。

---

## 8. 推荐执行顺序

```text
0. 建立 clean-room 规则与恢复基线
1. Web / Desktop capability inventory
2. Desktop native/domain boundary extraction
3. 建立 shared frontend architecture
4. 删除 Legacy Web
5. 删除 Legacy Desktop UI
6. 实现 Design System
7. 实现 Web Shell
8. 实现 Desktop Shell
9. 按 feature 同步实现核心业务
10. 接入 BeeCount Finance
11. Desktop native enhancement
12. Responsive / window-aware polish
13. Unit / integration / E2E / Rust / build regression
14. 文档同步
15. 全部通过后合并 main
```

---

## 9. Agent 执行约束

实际执行前应在仓库根目录建立或更新 `AGENTS.md`，明确：

```text
Frontend V2 is a clean-room rewrite.
Do not inspect or restore historical Web UI implementation.
Do not inspect or restore historical Desktop visual implementation.
Desktop native/domain contracts may be inspected only to preserve capability.
Derive functionality from backend/contracts/docs/tests/native capability.
Web and Desktop must share the V2 Design System and feature architecture.
Finance may reuse BeeCount Web implementation.
Do not stop at scaffolding; continue until functional regression and builds pass.
```

---

## 10. Definition of Done

V2 只有同时满足以下条件才允许合并 `main`：

```text
Web legacy UI removed
Desktop legacy UI removed
Desktop native capabilities preserved
Shared Design System in production use
Core features visually and behaviorally aligned
Finance integration complete
Web responsive verification complete
Desktop window/keyboard/native verification complete
TypeScript checks pass
Unit tests pass
Web E2E pass
Desktop smoke tests pass
Rust tests pass
Web build pass
Tauri build pass
No legacy CSS imported
Docs match implementation
```

最终目标不是分别获得一个“新 Web”和一个“新 Desktop”，而是建立一个真正统一的 **LifeTrace Frontend Platform V2**。