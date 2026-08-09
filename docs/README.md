# LifeTrace 文档目录规范

本目录统一采用稳定、简洁、可长期引用的文档命名方式，避免因版本号、重复项目前缀或不同 Agent 的命名习惯产生多份近似文件。

## 文件命名规则

- 普通 Markdown 文件统一使用**英文小写 kebab-case**，例如 `windows-release.md`。
- `docs/` 已经处于 LifeTrace 仓库内部，文件名不重复添加 `LifeTrace` 前缀。
- `epic-xx/` 目录内不重复添加 `EPICxx` 前缀。
- 产品规划、实施方案和长期维护文档的文件名及主标题不使用 `v1`、`v2`、`v3` 等版本后缀；直接更新同一个权威文档。
- 技术协议、API、Schema 或代码中的 `v1` 等版本语义可以保留，但不需要因此给普通说明文档增加版本后缀。例如文档使用 `sync-protocol.md`，正文仍可描述 Sync Protocol v1。
- 日期只用于需要保留历史快照的文档，例如 `project-completion-status-2026-08-09.md`。
- `README.md` 作为 GitHub 目录说明文件保留标准大写命名，是唯一常规例外。

## 根目录推荐名称

```text
docs/
├── README.md
├── roadmap.md
├── project-completion-status-YYYY-MM-DD.md
├── windows-release.md
├── epic-xx/
├── ui/
└── <feature-name>/
```

## Epic 目录推荐名称

同一类型文档统一使用固定职责名：

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

只创建当前 Epic 实际需要的文件，不要求每个目录包含全部类型。

## 命名选择原则

优先描述**文档职责**，而不是把项目名、Epic 名、作者、Agent、版本等元信息全部塞进文件名。

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
SYNC_PROTOCOL_V1.md
FINAL_NEW_PLAN_V2.md
```

如果同一主题已经存在权威文档，应更新原文件；只有历史快照、正式归档或语义确实不同的文档才新建文件。