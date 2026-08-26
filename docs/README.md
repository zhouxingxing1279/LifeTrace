# LifeTrace 文档目录规范

本目录统一采用稳定、简洁、可长期引用的文档命名方式。产品规划、实施方案和长期维护文档不使用 `v1`、`v2`、`v3` 等文件名后缀；同一主题优先更新权威文档。

## 文件命名规则

- 普通 Markdown 文件使用英文小写 kebab-case，例如 `windows-release.md`。
- `docs/` 内文件名不重复添加 `LifeTrace` 前缀。
- `epic-xx/` 目录内不重复添加 Epic 前缀。
- 技术协议/API/Schema 中的版本号可以保留，例如 Sync Protocol v1。
- 日期仅用于需要保留历史快照的文档，例如 `project-completion-status-2026-08-27.md`。
- `README.md` 作为目录入口保留标准大写命名。

## 根目录推荐结构

```text
docs/
├── README.md
├── roadmap.md
├── phase-2-roadmap.md
├── project-completion-status-YYYY-MM-DD.md
├── windows-release.md
├── docker-deployment.md
├── epic-xx/
└── <feature-name>/
```

UI 重构方案已经废弃并从仓库删除。后续若重新进行 Web/UI 大规模重构，应基于当时主干代码重新立项，不恢复或引用旧 `docs/ui/` 文档。

## Epic 目录推荐名称

```text
epic-xx/
├── implementation-plan.md
├── execution-plan.md
├── execution-report.md
├── completion-report.md
├── validation-report.md
├── implementation-status.md
├── architecture.md
├── migration-guide.md
└── adr/
```

只创建当前 Epic 实际需要的文件。

## 当前项目状态

- `docs/project-completion-status-2026-08-27.md`：**当前最新代码主干完成度基线**。项目进度判断优先参考此文件、各 Epic 的完成/验证报告和实际代码，而不是 Roadmap 中历史 checkbox。
- `docs/project-completion-status-2026-08-09.md`：历史快照，仅用于回看 8 月 9 日阶段状态。

## 产品规划入口

- `docs/roadmap.md`：LifeTrace 一期完整产品与开发路线图，描述目标范围和 Epic 边界。
- `docs/phase-2-roadmap.md`：LifeTrace 二期产品与开发路线图；当前主要规划 EPIC-33“英语学习闭环 2.0”。
- `docs/epic-33/README.md`：EPIC-33 产品范围与规划入口。

> Roadmap 的 `[ ]` 不作为当前实际完成状态的唯一依据。真实状态以主干代码、CI、`completion-report.md`、`validation-report.md` 和最新项目完成度快照为准。

## 核心基础设施文档

- `docs/epic-01/`：数据层重构、Schema、迁移、回滚与验证。
- `docs/epic-02/`：统一领域模型、实体注册表、同步协议、兼容策略与契约验证。
- `docs/epic-03/`：Rust/Axum + PostgreSQL Cloud 实施与验证。
- `docs/epic-04/`：认证、授权、设备和 Session 安全。
- `docs/epic-05/`：Windows 本地优先同步核心。
- `docs/epic-12/`：统一文件、附件与对象存储；包含 S3-compatible 签名传输、权限、去重和生命周期架构。
- `docs/epic-17/`：安全、隐私与数据生命周期。
- `docs/epic-19/`：客户端可观测性与诊断。
- `docs/epic-20/`：计划、任务、日历、等待事项和执行实体。

## 文件与相册

- `docs/epic-12/execution-plan.md`：统一云文件服务执行顺序和验收边界。
- `docs/epic-12/architecture.md`：文件元数据、对象存储、签名传输、去重和权限模型。
- `docs/epic-30/README.md`：本地加密相册/私密媒体保险库，严格 `local-only`。
- `docs/epic-31/README.md`：普通相册 Android ↔ Windows 设备直连同步规划。

EPIC-12 已完成服务端对象存储核心，但通用客户端缓存、缩略图、缓存清理和对象物理 GC 仍属于后续工作，不能仅因 Cloud API 已存在而标记整个跨端文件体系 100%。

## 邮件

邮件能力已经从 EPIC-27 基础聚合演进到 EPIC-32 Mail V2。当前实现包含统一 Inbox、未读聚合、按账号/文件夹导航、发件人聚合、邮件详情以及 Compose/Reply 等能力。

相关 Epic 文档仍按各目录中的执行/完成报告维护；旧项目状态报告中对邮件成熟度的描述已经过期。

## BeeCount 财务兼容

- `docs/beecount-cloud-integration/implementation-plan.md`：部署级协议兼容架构与数据边界。
- `docs/beecount-cloud-integration/deployment.md`：生产配置、验证、备份与客户端连接步骤。
- `docs/beecount-cloud-integration/validation-report.md`：自动化验证结果与环境限制。
- `docs/beecount-cloud-integration/phase-2-execution-plan.md`：只读财务适配器、鉴权隔离和接口契约。
- `docs/beecount-cloud-integration/phase-3-execution-plan.md`：BeeCount 原版客户端协议兼容、PostgreSQL 统一存储、迁移切换与回滚方案。
- `docs/beecount-cloud-integration/phase-4-execution-report.md`：附件/分类图标、WebSocket 实时同步与验证。
- `docs/beecount-cloud-integration/phase-5-execution-report.md`：Profile/头像、设备、共享账本成员/邀请/转让和共享同步权限。

购物订单子系统 / EPIC-29 已从主干移除，不再作为当前产品路线的一部分。

## 部署入口

- `docs/docker-deployment.md`：生产环境 Docker 镜像发布与升级流程。
- `docs/windows-release.md`：Windows 桌面版本发布流程（如当前仓库存在该文档）。

## 文档状态判定原则

评估项目进度时按以下证据优先级：

1. `main` 分支实际代码、Migration、API 和运行配置；
2. GitHub Actions / 构建门禁和测试结果；
3. `completion-report.md` / `validation-report.md`；
4. README 中明确声明的当前能力；
5. implementation/execution plan 和 Roadmap 仅用于确认目标，不直接证明已完成。

## 命名选择原则

优先描述文档职责，而不是把项目名、Epic、作者、Agent、版本等元信息全部放进文件名。

推荐：

```text
implementation-plan.md
validation-report.md
sync-protocol.md
windows-release.md
```

避免：

```text
LifeTrace_EPIC05_Agent_Implementation_Plan.md
LifeTrace_Complete_Roadmap_v3.md
FINAL_NEW_PLAN_V2.md
```

如果同一主题已经存在权威文档，应更新原文件；只有历史快照、正式归档或语义确实不同的文档才创建新文件。
