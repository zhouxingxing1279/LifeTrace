# LifeTrace

> 一个以 **local-first** 为核心的个人管理平台，把坚持、英语、健身、财务、笔记、照片和 AI 助手放进同一套长期可积累的数据系统中。

LifeTrace 目前以 **Windows 桌面端**为主要使用入口，同时提供面向 LifeTrace Cloud 的**浏览器客户端**。项目采用 Monorepo：Desktop、Cloud、共享契约和部署配置位于同一个仓库，但保持独立的源码、依赖、构建和发布边界。

当前桌面版本：`0.2.1`

## 项目目标

LifeTrace 不是单一的打卡工具，而是一个长期个人数据系统。设计重点包括：

- **Local-first**：桌面端核心数据优先保存在本地 SQLite，日常使用不依赖云端服务。
- **统一记录**：坚持、训练、阅读、消费、笔记等信息进入同一个个人管理平台，而不是散落在多个应用中。
- **长期反馈**：不仅记录“做没做”，还要支持趋势、复盘、统计和后续 AI 分析。
- **桌面软件体验**：Tauri 原生桌面壳层、右键菜单、快捷操作、确认流程、自动更新能力和本地文件能力。
- **隐私边界明确**：照片、私密相册、局域网上传等本地能力不暴露给浏览器客户端。
- **云端可选**：Desktop 与 Cloud 通过稳定契约同步，不要求必须同时部署或同时发布。

## 当前功能

| 模块 | 当前能力 |
| --- | --- |
| **个人总览** | 汇总今日坚持、本周训练、本月支出、总资产、最近训练、最近账单和笔记 |
| **AI 管家** | 基于个人记录进行查询、总结与回顾；支持独立 AI 配置 |
| **坚持项目** | 自定义项目、目标与记录，查看完成情况和长期趋势 |
| **每日英语** | 英文阅读、阅读总结、高亮、生词、阅读记录与统计 |
| **健身训练** | 训练历史、训练数据导入、训记数据接入及手机上传入口 |
| **财务管理** | 财务概览、账户、流水、分类及微信/支付宝账单导入 |
| **笔记** | 长期笔记、编辑、文件夹/标签组织、搜索与桌面快捷操作 |
| **生活日历** | 按日期查看跨模块生活记录 |
| **每日复盘** | 对当天记录进行快速回顾和总结 |
| **照片** | 本地照片同步与媒体管理，桌面端支持局域网手机上传 |
| **私密相册** | 独立密码、本地加密存储、自动锁定、回收站、完整性检查；密码丢失后不提供恢复机制 |
| **云端 / 浏览器** | 浏览器端通过 LifeTrace Cloud 使用适合云端运行的业务模块，不复制桌面本地数据库 |

> 浏览器端不会包含照片同步、私密相册、局域网上传、本地密钥和设备文件路径等桌面专属能力。

## 架构

```text
                         LifeTrace
                            │
          ┌─────────────────┴─────────────────┐
          │                                   │
   Desktop / Windows                    Browser Client
   Tauri 2 + React                      React + Vite
          │                                   │
          │                                   │ HTTP
          ▼                                   ▼
   Local SQLite                         LifeTrace Cloud
   Local files / vault                  Rust + Axum
   Native capabilities                       │
          │                                   ▼
          │                              PostgreSQL
          │                                   │
          └──────── Shared contracts ─────────┘
                     crates/contracts
```

### Desktop

桌面应用位于 `apps/desktop/`：

- React 19 + TypeScript + Vite
- Tauri 2
- Rust 本地能力层
- SQLite（`rusqlite`）
- 本地照片与局域网上传服务
- AES-GCM + Argon2 私密相册
- Tauri Updater / Dialog / Process 等原生插件

### Cloud

云端服务位于 `services/cloud/`：

- Rust
- Axum
- Tokio
- PostgreSQL
- SQLx
- 账号、设备、会话、同步和浏览器 API

### Shared Contracts

Desktop 与 Cloud 不直接依赖彼此内部实现，跨端边界通过共享协议连接：

- `crates/lifetrace-contracts/`
- `crates/lifetrace-sync-client/`
- `contracts/`

这样可以独立升级 Desktop 与 Cloud，同时通过契约检查控制兼容性。

## 数据与隐私

### 桌面端

桌面端遵循 local-first 原则：业务数据主要保存在本地 SQLite，本地文件和桌面原生能力由 Tauri/Rust 层处理。

私密相册与普通照片功能隔离：

- 使用独立密码解锁；
- 密码派生使用 Argon2；
- 加密数据使用 AES-GCM；
- 解锁后的敏感对象 URL 在锁定/离开后清理；
- 支持窗口失焦/离开页签自动锁定；
- 支持密文完整性检查；
- 初始化时明确要求确认“密码丢失后无法恢复”。

### 浏览器端

浏览器客户端是 LifeTrace Cloud 的在线客户端，不是桌面本地数据库的 Web 镜像：

- 不使用 IndexedDB 保存业务主数据；
- 不提供离线写入队列；
- 写操作以 Cloud 确认结果为准；
- 桌面专属的本地文件、密钥、照片和局域网能力不会进入浏览器包。

更详细的边界说明见 [`docs/browser-web.md`](docs/browser-web.md)。

## 快速开始

### 环境要求

开发 Desktop 至少需要：

- Node.js `>= 22.13.0`
- Rust stable
- Tauri 2 对应的平台构建环境

Windows 开发机还需要满足 Tauri 的 Windows 编译依赖。

### 安装 Desktop 依赖

```powershell
npm ci --prefix apps/desktop
```

### 启动桌面端

从仓库根目录：

```powershell
npm run dev
```

或者：

```powershell
cd apps/desktop
npm run dev
```

### 启动浏览器端与 Cloud

本地启动 PostgreSQL / Cloud：

```powershell
docker compose -f deploy/cloud/docker-compose.local.yml --profile cloud up -d --build
```

启动浏览器客户端：

```powershell
npm run browser:dev
```

默认开发访问地址：

```text
http://127.0.0.1:4173
```

需要显式指定 Cloud 地址时：

```powershell
$env:VITE_LIFETRACE_CLOUD_URL="http://127.0.0.1:8787"
npm run browser:dev
```

## 常用命令

以下命令均可在仓库根目录执行：

| 命令 | 用途 |
| --- | --- |
| `npm run dev` | 启动 Tauri Desktop 开发环境 |
| `npm run lint` | TypeScript 类型检查 |
| `npm run test:unit` | 运行前端单元测试 |
| `npm run test:desktop` | Desktop 类型检查 + 单测 + 两套 Web 构建 |
| `npm run test:rust` | 运行 Desktop Rust 回归测试 |
| `npm run web:build` | 构建 Tauri Web 前端 |
| `npm run browser:build` | 构建浏览器客户端 |
| `npm run build:desktop` | 构建 Tauri Desktop |
| `npm run dev:cloud` | 本地启动 Cloud 服务 |
| `npm run test:cloud` | 运行 Cloud Rust 测试 |
| `npm run build:cloud` | 构建 Cloud Release |
| `npm run contracts:check` | 重新生成并检查共享契约是否一致 |
| `npm run test:all` | Desktop + Cloud + Contracts 全量验证 |

## 测试与质量门禁

当前仓库的主要验证链包括：

1. TypeScript 类型检查；
2. 前端单元测试；
3. Tauri Web 构建；
4. Browser Web 构建；
5. Desktop Rust 回归测试；
6. Cloud Rust 测试；
7. 共享契约一致性检查；
8. Windows Tauri 可执行文件构建验证。

客户端同时包含 Error Boundary、客户端日志与错误诊断基础设施，用于避免请求发出前异常、原生 API 调用错误等问题被业务层错误提示吞掉后难以定位。

## 仓库结构

```text
LifeTrace/
├─ apps/
│  └─ desktop/                 # Tauri + React + SQLite + Browser UI
├─ services/
│  └─ cloud/                   # Rust + Axum + PostgreSQL
├─ crates/                     # Desktop / Cloud 共享 Rust crates
├─ contracts/                  # API / 同步契约及生成类型
├─ deploy/                     # Cloud / PostgreSQL 部署配置
├─ design-system/              # LifeTrace 视觉与交互规范
├─ docs/                       # Roadmap、执行方案、设计与发布文档
├─ scripts/                    # 仓库级开发 / 发布脚本
├─ tools/                      # 契约生成等开发工具
├─ package.json                # Monorepo 统一命令入口
└─ README.md
```

## UI 与桌面交互

LifeTrace 的用户端 UI 正在从“网页式 Dashboard”收敛为桌面软件体验。目前已经建立：

- 统一字号、间距、颜色、圆角和控件 Token；
- 更清晰的信息层级，减少长期常驻的小字提示；
- 统一 `Action` 模型；
- 业务对象右键菜单；
- `···` 更多菜单；
- 危险操作确认流程；
- 菜单键盘导航和边界定位；
- Toast / Dialog / Context Menu 等统一交互基础设施。

实施记录见 [`docs/ui-redesign/IMPLEMENTATION_REPORT.md`](docs/ui-redesign/IMPLEMENTATION_REPORT.md)。

## Windows 发布

Desktop 与 Cloud 独立发布。

Windows Desktop 使用 Tauri 构建，仓库中已经包含 Windows Release / Updater 的自动化流程和版本一致性检查。安装包、签名及更新元数据的完整发布方式见：

[`docs/releasing-windows.md`](docs/releasing-windows.md)

## 主要文档

- [完整 Roadmap](docs/LifeTrace_Complete_Roadmap_v2.md)
- [浏览器端架构与边界](docs/browser-web.md)
- [UI 重构执行方案](docs/ui-redesign/EXECUTION_PLAN.md)
- [UI 重构实施报告](docs/ui-redesign/IMPLEMENTATION_REPORT.md)
- [本地加密相册设计](docs/local-encrypted-album/)
- [成长树系统规划](docs/growth-tree-system/)
- [Windows 发布与在线更新](docs/releasing-windows.md)

## Roadmap 与当前实现的关系

`docs/` 中包含大量后续设计和 EPIC 规划。**设计文档存在不代表功能已经进入当前版本。**

README 的“当前功能”以 `main` 分支现有代码为准；成长树、更完整的长期成长模型以及后续 AI 能力等仍按照 Roadmap 分阶段演进。

## Sparse Checkout

虽然 LifeTrace 是一个 Monorepo，但可以只检出需要的部分。

Desktop：

```bash
git clone --filter=blob:none --no-checkout https://github.com/zhouxingxing1279/LifeTrace.git
cd LifeTrace
git sparse-checkout init --cone
git sparse-checkout set apps/desktop crates contracts
git checkout main
```

Cloud：

```bash
git clone --filter=blob:none --no-checkout https://github.com/zhouxingxing1279/LifeTrace.git
cd LifeTrace
git sparse-checkout init --cone
git sparse-checkout set services/cloud crates contracts deploy/cloud scripts/cloud
git checkout main
```

## 发布边界

- **Desktop**：Windows / Tauri 应用，拥有独立版本与发布流程。
- **Cloud**：Rust 服务 / Docker 部署，拥有独立发布周期。
- **Contracts**：控制 Desktop、Browser 与 Cloud 之间的协议兼容。

Desktop 与 Cloud 不要求部署在同一台机器，也不要求同时发布。