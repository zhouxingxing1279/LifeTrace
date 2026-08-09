# LifeTrace EPIC-20 个人执行系统详细执行计划

> EPIC：EPIC-20「计划、任务、日历、等待事项与备忘录」  
> 状态：Ready for Implementation  
> 更新日期：2026-08-08  
> 目标仓库：`zhouxingxing1279/LifeTrace`  
> 目标目录：`docs/epic-20/`  
> 依据：`docs/LifeTrace_Complete_Roadmap_v2.md` 中 EPIC-20，以及当前 LifeTrace 本地优先、离线可用、统一领域服务、增量同步和全链路可观测架构。

---

## 1. EPIC 目标

EPIC-20 的目标不是简单增加一个 Todo List，而是建立 LifeTrace 的统一「个人执行层」，把以下问题放进同一套领域模型和交互闭环：

1. **计划做什么**：项目、任务、子任务、前置依赖、优先级、预计耗时。
2. **什么时候做**：截止时间、计划时间、日历事件、全天事件、重复事件、提醒。
3. **正在等什么**：等待某人回复、等待外部事项完成、预计回复时间和跟进提醒。
4. **临时记住什么**：通过备忘录快速记录短信息，并可在需要时转化为行动。
5. **最后结果是什么**：记录实际完成时间、完成结果、总结及关联文件/笔记/邮件。
6. **跨设备如何保持一致**：离线可创建和修改，恢复联网后通过现有同步体系增量同步。
7. **以后 AI 如何安全操作**：所有写操作必须经过统一 Domain Service，不允许 AI 或页面直接绕过领域校验写数据库。

最终用户应能够形成如下闭环：

```text
想法 / 临时信息
      ↓
    备忘录
      ↓（必要时转换）
任务 / 日历事件 / 等待事项
      ↓
提醒、执行、跟进
      ↓
完成 / 解决
      ↓
结果记录 + 关联资料
      ↓
时间线 / 搜索 / 后续 AI 查询与操作
```

---

## 2. 本 EPIC 的范围

### 2.1 必须完成

- 项目与任务模型
- 一次性任务
- 重复任务
- 子任务
- 任务前置依赖
- 优先级
- 预计耗时与实际耗时
- 截止时间
- 任务计划时间
- 任务提醒
- 任务状态流转
- 等待事项
- 等待事项跟进提醒
- 等待事项转任务
- 内部日历事件
- 全天事件
- 定时事件
- 重复事件
- 时区处理
- 时间冲突提示
- 任务安排到日历
- **备忘录**
- **备忘录提醒**
- **备忘录置顶、归档、标签**
- **备忘录一键转任务 / 日历事件 / 等待事项**
- 通用提醒能力
- 完成结果记录
- 关联文件 / 笔记 / 邮件等实体
- SQLite 本地持久化
- PostgreSQL 云端完整副本
- 同步协议接入
- 离线操作与恢复联网后的同步
- 桌面端 UI
- 日志、错误码、审计与自动化测试

### 2.2 明确不在本 EPIC 中完成

以下能力只预留领域接口，不在 EPIC-20 内提前实现：

- AI 自然语言创建/修改任务：属于 EPIC-21/22/24 后续能力。
- AI 自动决定什么时候提醒：不在本 EPIC 自动决策。
- 统一输入收件箱：属于 EPIC-26。
- 邮件抓取、邮件摘要、邮件自动转任务：属于 EPIC-27。
- Google Calendar / Outlook Calendar 等第三方日历双向同步：当前 EPIC 只实现 LifeTrace 内部日历。
- 团队协作、多人任务分配、共享日历：LifeTrace 当前仍按个人系统设计。
- 完整知识库编辑器：继续由 Notes 模块负责，备忘录不替代 Notes。

---

## 3. 核心设计原则

### 3.1 本地优先

所有用户主动创建、编辑、完成、归档等操作先写入本地 SQLite，在本地立即生效，再进入 Sync Outbox。断网时核心功能不得失效。

```text
Desktop UI
   ↓
Domain Service
   ↓
Local Repository / SQLite
   ↓
Sync Outbox
   ↓
Cloud Sync API
   ↓
PostgreSQL
```

禁止把云端 API 成功作为本地操作成功的前置条件。

### 3.2 所有入口使用同一领域服务

以下入口未来必须共用同一套业务规则：

```text
页面操作
快捷键
Android / Web
AI Tool
邮件转任务
统一输入
      ↓
Execution Domain Service
      ↓
Repository
```

页面不能为了“开发快”直接拼 SQL；AI 也不能直接操作数据库。

### 3.3 五类核心实体职责明确

| 实体 | 核心含义 | 是否代表行动 | 是否占用日历时间 | 是否依赖外部结果 |
|---|---|---:|---:|---:|
| Task | 我要完成的行动 | 是 | 可选 | 可选 |
| CalendarEvent | 某时间段发生的事项 | 可选 | 是 | 否 |
| WaitingItem | 当前行动依赖别人或外部事项 | 否 | 否 | 是 |
| Memo | 我要暂时记住的短信息 | 否 | 否 | 否 |
| Note | 长期保存的结构化知识/内容 | 否 | 否 | 否 |

其中 **Memo 必须独立于 Note 和 Task**，不能把备忘录简单实现成特殊 Task，也不能强制为每条备忘录创建完整 Notes 富文本记录。

---

## 4. 备忘录功能定义

备忘录是本次 EPIC 在原路线图基础上的正式扩展能力。

### 4.1 产品定位

备忘录用于「快速记一下，以免忘记」，强调低操作成本：

- 输入一句或几句文字即可保存。
- 默认不需要截止时间。
- 默认不需要任务状态。
- 默认不进入日历。
- 可以设置一个提醒时间。
- 可以置顶。
- 可以添加标签/上下文。
- 可以归档。
- 可以搜索。
- 可以关联其他 LifeTrace 实体。
- 当信息变成行动时，一键转成任务、日历事件或等待事项。

### 4.2 Memo 与 Notes 的边界

**Memo：**

- 快速创建。
- 内容较短。
- 轻量文本为主。
- 适合“记得买滤芯”“下次问导师这个问题”“周末研究一下某工具”等临时信息。
- 可选提醒。
- 支持置顶和归档。

**Note：**

- 长文档。
- Tiptap / Markdown / HTML 等富内容。
- 文件夹、版本、复杂编辑。
- 适合长期知识沉淀。

当 Memo 内容不断扩展时，可增加“转为笔记”的入口，但这不是 EPIC-20 的硬性阻塞项；至少要预留通过 EPIC-13 关联原 Memo 与新 Note 的能力。

### 4.3 Memo 转换原则

转换不是简单复制后丢弃原记录，必须保留来源链路：

```text
Memo A
  ├── convert_to → Task B
  ├── convert_to → CalendarEvent C
  └── convert_to → WaitingItem D
```

默认转换行为：

- 创建目标实体。
- 创建 `converted_to` / `derived_from` 关联。
- 原 Memo 默认归档，而不是物理删除。
- 转换失败时保持原 Memo 不变。
- 重试必须幂等，不能产生多个重复 Task/Event/WaitingItem。

---

## 5. 领域模型设计

### 5.1 Project

建议字段：

```text
id
name
description
status
color/icon（可选 UI 元数据）
sort_order
version
created_at
updated_at
deleted_at
modified_by_device
```

项目只负责组织任务，不在本 EPIC 引入复杂项目管理方法论。

### 5.2 Task

建议字段：

```text
id
project_id nullable
parent_task_id nullable
title
description
status               todo | in_progress | waiting | done | cancelled
priority             low | normal | high | urgent
estimated_minutes nullable
actual_minutes nullable
due_at nullable
scheduled_start_at nullable
scheduled_end_at nullable
timezone nullable
context nullable
completed_at nullable
cancelled_at nullable
recurrence_rule_id nullable
completion_result_id nullable
version
created_at
updated_at
deleted_at
modified_by_device
```

要求：

- `parent_task_id` 支持子任务。
- 子任务不得形成环。
- 父任务完成策略第一版采用“允许完成，但 UI 明确提示仍有未完成子任务”；不要隐式批量完成子任务。
- `actual_minutes` 可以由用户手工填写；自动计时不是 EPIC-20 必需能力。

### 5.3 TaskDependency

```text
id
task_id
depends_on_task_id
dependency_type
created_at
```

第一版至少支持 `finish_before_start` 语义。

必须校验：

- 不允许自己依赖自己。
- 不允许形成有向环。
- 前置任务未完成时，允许查看/编辑后续任务，但执行态必须提示阻塞原因。

### 5.4 RecurrenceRule

任务和日历事件应共享统一重复规则语义，不要各自实现一套字符串解析器。

第一版至少支持：

- 每天
- 每周指定星期
- 每月指定日期
- 每 N 天 / 周 / 月
- 截止某日期
- 最大发生次数
- 永久重复

建议内部使用可验证的统一结构，并提供与 RFC 5545 RRULE 的明确映射能力；不要让 UI 直接拼自由字符串。

重复任务采用“规则 + occurrence 实例”模型：

- 完成某一次 occurrence 不等于结束整个重复任务。
- 支持跳过本次。
- 支持只编辑本次。
- 支持编辑本次及未来。
- 修改规则不能破坏已经完成的历史 occurrence。

### 5.5 WaitingItem

建议字段：

```text
id
title
description
status               open | resolved | cancelled
waiting_for           人 / 事项描述
expected_at nullable
follow_up_at nullable
resolved_at nullable
resolution_summary nullable
source_task_id nullable
version
created_at
updated_at
deleted_at
modified_by_device
```

WaitingItem 必须可以独立存在，也可以从 Task 的 `waiting` 状态创建。

### 5.6 CalendarEvent

建议字段：

```text
id
title
description
is_all_day
start_at nullable
end_at nullable
start_local_date nullable
end_local_date nullable
timezone
status               scheduled | cancelled
recurrence_rule_id nullable
source_task_id nullable
version
created_at
updated_at
deleted_at
modified_by_device
```

规则：

- 全天事件使用 local natural date，不允许简单以 `00:00 UTC` 代替。
- 定时事件保存 UTC 时间点 + 原始 timezone。
- `end_at >= start_at`。
- 取消事件保留历史，不直接删除。

### 5.7 Memo

建议字段：

```text
id
content
plain_text
is_pinned
status               active | archived
archived_at nullable
reminder_id nullable
context nullable
version
created_at
updated_at
deleted_at
modified_by_device
```

第一版不建议给 Memo 增加复杂富文本 Schema；如需要简单格式，可使用轻量 Markdown，但必须保证纯文本搜索字段可直接索引。

### 5.8 Reminder

提醒必须抽成共享模型：

```text
id
subject_type         task | calendar_event | waiting_item | memo
subject_id
trigger_at
timezone
status               scheduled | fired | dismissed | cancelled
snoozed_until nullable
last_fired_at nullable
fire_key              幂等键
version
created_at
updated_at
deleted_at
modified_by_device
```

共享提醒服务负责：

- 新建提醒
- 修改时间
- 取消提醒
- 到期触发
- 稍后提醒
- 已读/关闭
- 重启应用后恢复调度
- 多次扫描不重复通知

禁止在 Task、Waiting、Calendar、Memo 四个模块分别写四套计时器。

### 5.9 CompletionResult

```text
id
task_id
summary
completed_at
actual_minutes nullable
created_at
updated_at
```

附件、Note、Email 不直接塞进结果表 JSON，而通过 EPIC-13 的跨实体关联模型保存。

---

## 6. 状态机

### 6.1 Task

```text
todo
 ├──→ in_progress
 ├──→ waiting
 ├──→ done
 └──→ cancelled

in_progress
 ├──→ waiting
 ├──→ done
 └──→ cancelled

waiting
 ├──→ in_progress
 ├──→ done
 └──→ cancelled
```

要求：

- `done` 必须写 `completed_at`。
- `cancelled` 必须写 `cancelled_at`。
- 从 done 重新打开时保留历史审计信息，但当前 `completed_at` 按领域规则重置/版本化。
- 状态变更必须走 Domain Service。

### 6.2 WaitingItem

```text
open → resolved
open → cancelled
```

`resolved` 后允许创建后续 Task，但不能修改成一个新的 open 事项来复用旧 ID。

### 6.3 Memo

```text
active → archived
archived → active
```

删除仍使用全局软删除机制。

Memo 没有 `done`，因为它不是任务。

---

## 7. 时间与时区设计

EPIC-20 是 LifeTrace 中时间语义最密集的模块之一，必须在编码前固定规则。

### 7.1 时间分类

至少区分：

1. **绝对时间点**：提醒时间、定时事件开始结束、任务 scheduled time。
2. **本地自然日**：全天事件、只指定“某一天截止”的任务。
3. **时区**：创建事件时的 IANA timezone。
4. **重复规则时区**：例如“每天早上 09:00 America/Los_Angeles”不能被转换为固定 UTC 间隔。

### 7.2 DST 测试必须覆盖

- 夏令时开始日缺失小时。
- 夏令时结束日重复小时。
- 用户切换系统时区。
- 跨时区查看事件。
- 全天事件不得因时区变化变成前一天/后一天。

---

## 8. 任务与日历的关系

Task 与 CalendarEvent 不能合并成同一张表。

任务可以只有截止时间而没有日历时间；日历事件也可能不是任务。

“安排任务到日历”应创建关联：

```text
Task
  └── scheduled_as → CalendarEvent
```

要求：

- Task 的业务状态和 CalendarEvent 的时间安排独立。
- 移动日历事件不能自动改变任务 due_at，除非用户明确选择同步修改。
- 删除/取消日历安排不能删除 Task。
- 完成 Task 可提示是否保留已发生事件，不自动删除历史事件。

---

## 9. 等待事项设计

等待事项用于解决“我现在没法继续做，但必须记得之后跟进”的问题。

### 9.1 创建入口

- 独立新建 WaitingItem。
- Task 进入 waiting 状态时选择创建 WaitingItem。
- Memo 转 WaitingItem。
- EPIC-27 后续从 Email 创建 WaitingItem。

### 9.2 等待总览

至少提供：

- 全部等待中
- 今天需要跟进
- 已过预计时间
- 本周预计返回
- 已解决

### 9.3 转任务

WaitingItem 一键转 Task 时：

- 默认使用等待事项标题作为任务标题。
- 用户确认/编辑截止时间和优先级。
- 建立 `derived_from` 关联。
- WaitingItem 可选择直接标记 resolved，默认不删除。

---

## 10. 通用关联模型接入

必须复用 EPIC-13 的跨实体关联模型；如果 EPIC-13 尚未提供最终 API，则本 EPIC 只能增加兼容适配层，不能自行创造第二套永久关联体系。

EPIC-20 至少支持以下关系：

```text
Task ↔ Note
Task ↔ File
Task ↔ Email
Task ↔ CalendarEvent
Task ↔ WaitingItem
Task ↔ Memo
WaitingItem ↔ Contact
WaitingItem ↔ Email
Memo ↔ Note
Memo ↔ File
Memo → Task / Event / WaitingItem
CompletionResult ↔ Note / File / Email
```

关联删除与业务实体删除分离。

---

## 11. Domain Service 设计

建议新增统一 execution domain，而不是分别让页面调用 Repository。

建议命令接口：

```text
ProjectService
- create_project
- update_project
- archive_project

TaskService
- create_task
- update_task
- change_task_status
- complete_task
- reopen_task
- cancel_task
- add_subtask
- add_dependency
- remove_dependency
- schedule_task
- set_task_recurrence

WaitingService
- create_waiting_item
- update_waiting_item
- resolve_waiting_item
- cancel_waiting_item
- convert_waiting_to_task

CalendarService
- create_event
- update_event
- cancel_event
- set_event_recurrence
- move_event
- list_conflicts

MemoService
- create_memo
- update_memo
- pin_memo
- unpin_memo
- archive_memo
- restore_memo
- set_memo_reminder
- convert_memo_to_task
- convert_memo_to_event
- convert_memo_to_waiting

ReminderService
- create_reminder
- reschedule_reminder
- snooze_reminder
- dismiss_reminder
- cancel_reminder
- poll_due_reminders

ExecutionQueryService
- get_today_overview
- get_upcoming
- get_waiting_overview
- get_calendar_range
- get_memo_list
- get_task_detail
```

所有写命令统一负责：

1. 参数校验。
2. 权限/数据所有权校验。
3. 领域状态校验。
4. 数据库事务。
5. 写 Sync Outbox。
6. 写 Audit Log。
7. 发送领域事件。
8. 返回可序列化 DTO。

---

## 12. 本地数据库实现

当前桌面端已有 `apps/desktop/src-tauri/src/database`，EPIC-20 的 SQLite migration 和 repository 优先放在现有数据库体系内，不再创建旁路数据库。

建议目录结构（最终以现有模块组织方式为准）：

```text
apps/desktop/src-tauri/src/
├── database/
│   ├── migrations/
│   └── repositories/
├── execution/
│   ├── mod.rs
│   ├── models.rs
│   ├── task_service.rs
│   ├── waiting_service.rs
│   ├── calendar_service.rs
│   ├── memo_service.rs
│   ├── reminder_service.rs
│   └── query_service.rs
└── sync/
```

### 12.1 Migration 要求

新增表时必须：

- 使用现有 migration 框架。
- migration 可重复检测，不允许半执行状态。
- 迁移前后运行 schema 校验。
- 为常用过滤字段建立索引。
- 所有可同步实体带 EPIC-02 规定的 version / timestamps / tombstone / device 字段。

### 12.2 推荐索引

至少评估：

```text
tasks(status, due_at)
tasks(project_id, status)
tasks(parent_task_id)
calendar_events(start_at, end_at)
waiting_items(status, follow_up_at)
waiting_items(status, expected_at)
memos(status, is_pinned, updated_at)
reminders(status, trigger_at)
```

Memo 搜索使用项目已有全文搜索方案；在没有统一 FTS 前，至少维护 `plain_text`，不要依赖解析富文本才能搜索。

---

## 13. Contracts 与同步协议

当前仓库已经存在：

```text
crates/lifetrace-contracts
crates/lifetrace-sync-client
```

EPIC-20 必须把以下实体加入共享合同：

```text
project
task
task_dependency
recurrence_rule
waiting_item
calendar_event
memo
reminder
completion_result
```

### 13.1 Sync entity_type

使用 EPIC-02 的统一命名和协议版本机制，不允许前后端自行硬编码互不一致的字符串。

### 13.2 同步要求

每个实体至少覆盖：

- create
- update
- soft delete
- pull
- push
- duplicate change_id 幂等
- base_version 冲突
- tombstone
- snapshot restore

### 13.3 冲突原则

禁止依赖客户端本地时间判断“最后修改者”。

建议：

- 普通标量字段按照服务端版本冲突流程处理。
- 状态流转冲突优先返回明确 conflict，由领域层重新决策。
- 重复规则发生并发修改时不要自动字段级 merge。
- Memo 文本发生并发编辑时，第一版宁可保留冲突副本/提示用户选择，也不要静默覆盖内容。
- complete/cancel/resolve 等命令必须幂等。

---

## 14. 云端实现

当前云端位于 `services/cloud`，需要为 EPIC-20 增加 PostgreSQL migration、repository/domain handler 和同步实体映射。

要求：

- 云端保存完整业务副本，而不仅是 outbox 日志。
- 云端使用和本地一致的 ID。
- 服务端负责 authoritative version/cursor。
- 数据写入与 change log 必须保持事务一致性。
- 对重复请求使用 change_id 保证幂等。
- 删除使用 tombstone，满足其他设备增量拉取。

本 EPIC 不要求云端成为任务执行的唯一入口；桌面离线仍然必须完整可用。

---

## 15. 桌面端 UI 信息架构

建议在个人执行模块中提供以下一级视图：

```text
执行
├── 今天
├── 任务
├── 日历
├── 等待
└── 备忘录
```

### 15.1 今天

聚合但不复制数据：

- 今天计划执行的任务
- 今天截止的任务
- 已逾期任务
- 今天日历事件
- 今天需要跟进的等待事项
- 今天触发提醒的备忘录

### 15.2 任务页

至少支持：

- 列表视图
- 项目筛选
- 状态筛选
- 优先级筛选
- 截止时间筛选
- 标签/上下文筛选
- 子任务展开
- 依赖阻塞标识
- 重复任务标识
- 快速完成
- 右键菜单

右键菜单建议包含：

- 开始执行
- 标记完成
- 转为等待
- 安排到日历
- 修改截止时间
- 设置提醒
- 添加子任务
- 复制任务
- 取消
- 删除

### 15.3 日历页

第一版建议包含：

- 月视图
- 周视图
- 日视图
- 全天区域
- 定时事件区域
- Task schedule 展示
- 拖动调整时间
- 冲突提示
- 重复事件标识

### 15.4 等待页

布局重点不是普通 Todo，而是“跟进”：

- 等待对象/事项
- 已等待时长
- 预计返回时间
- 下次跟进时间
- 是否逾期
- 关联任务/邮件
- 解决按钮
- 转任务按钮

### 15.5 备忘录页

备忘录 UI 采用快速、低干扰设计：

- 顶部快速输入框。
- `Enter/Ctrl+Enter` 快速保存规则统一。
- 置顶区。
- 最近备忘录。
- 有提醒的备忘录。
- 已归档。
- 搜索。
- 标签筛选。

每条 Memo 的主要操作：

```text
编辑
置顶/取消置顶
设置提醒
转任务
转日历事件
转等待事项
关联笔记/文件
归档
删除
```

Memo 卡片不要加入任务式的复选框，以免产品语义混淆。

---

## 16. 提醒调度

### 16.1 调度原则

本地提醒由统一 Reminder Scheduler 管理：

```text
App 启动
  ↓
读取 scheduled reminder
  ↓
注册最近需要触发的提醒
  ↓
到点
  ↓
生成系统通知 / 应用内通知
  ↓
记录 fired + fire_key
```

### 16.2 重启恢复

必须测试：

- 创建提醒后关闭应用。
- 提醒时间前重新启动。
- 应用读取持久化记录并恢复调度。
- 已 fired 的提醒不得重复弹出。

### 16.3 离线与多设备

第一阶段以“每台设备根据已同步的 reminder 自己调度”为原则。

同一个 reminder 在多设备触发属于产品可接受行为；后续若接入统一 Push 服务，再设计 server-side delivery 去重，不要在本 EPIC 引入复杂分布式通知系统。

---

## 17. 时间冲突检查

CalendarService 提供 `list_conflicts(start, end, exclude_event_id)`。

第一版冲突属于**警告，不是硬阻塞**：

- 两个定时事件重叠。
- Task schedule 与事件重叠。
- 用户可以确认后仍然保存。

全天事件不参与普通小时级冲突阻塞。

---

## 18. 可观测性与审计

EPIC-20 依赖 EPIC-19 的客户端可观测能力。

建议结构化事件：

```text
execution.task.created
execution.task.status_changed
execution.task.completed
execution.waiting.created
execution.waiting.resolved
execution.calendar.created
execution.calendar.updated
execution.memo.created
execution.memo.converted
execution.reminder.fired
execution.reminder.failed
execution.sync.conflict
```

日志字段允许：

```text
entity_type
entity_id
operation
result
error_code
duration_ms
version
sync_state
```

禁止日志记录：

- Memo 正文
- Task 描述全文
- Note 内容
- Email 正文
- 其他隐私文本

写操作必须进入 Audit Log，至少记录谁/哪个设备、什么时间、对哪个实体执行了什么动作。

---

## 19. 错误码

建议在共享 contracts 中定义稳定错误码，至少覆盖：

```text
EXECUTION_VALIDATION_FAILED
TASK_INVALID_STATE_TRANSITION
TASK_DEPENDENCY_CYCLE
TASK_BLOCKED_BY_DEPENDENCY
RECURRENCE_RULE_INVALID
WAITING_ALREADY_RESOLVED
CALENDAR_INVALID_RANGE
CALENDAR_TIME_CONFLICT
MEMO_ALREADY_ARCHIVED
MEMO_CONVERSION_FAILED
REMINDER_INVALID_TRIGGER
REMINDER_ALREADY_FIRED
ENTITY_VERSION_CONFLICT
ENTITY_NOT_FOUND
SYNC_CONFLICT
```

UI 不直接显示 Rust/SQL 原始错误，应映射为用户可理解信息，同时保留 correlation/request 信息供日志排查。

---

## 20. Agent 执行阶段

下面顺序是本 EPIC 的强制实施顺序。后续 Agent 应按阶段完成、测试和提交，不建议一次性修改所有层。

### Phase 0：现状审计与技术设计冻结

- [ ] 盘点现有 SQLite migration 机制。
- [ ] 盘点现有 PostgreSQL migration 机制。
- [ ] 盘点 Sync entity 注册方式。
- [ ] 盘点 `lifetrace-contracts` DTO/枚举模式。
- [ ] 盘点桌面端 service/store/UI 组织方式。
- [ ] 盘点 EPIC-13 association API 是否已经可用。
- [ ] 盘点 EPIC-19 logging/audit API。
- [ ] 输出 EPIC-20 schema ER 图。
- [ ] 固定 Task/Waiting/Memo 状态机。
- [ ] 固定时间和时区规则。
- [ ] 固定 recurrence 结构。
- [ ] 确认所有新实体的 `entity_type`。

**完成标准：** 数据结构、状态机、时间语义在写 migration 前冻结，不边写 UI 边临时改 Schema。

### Phase 1：Contracts + Schema 基础

- [ ] 在 `crates/lifetrace-contracts` 增加 EPIC-20 DTO / enum。
- [ ] 增加 Project contract。
- [ ] 增加 Task contract。
- [ ] 增加 TaskDependency contract。
- [ ] 增加 RecurrenceRule contract。
- [ ] 增加 WaitingItem contract。
- [ ] 增加 CalendarEvent contract。
- [ ] 增加 Memo contract。
- [ ] 增加 Reminder contract。
- [ ] 增加 CompletionResult contract。
- [ ] 增加错误码。
- [ ] 创建本地 SQLite migrations。
- [ ] 创建云端 PostgreSQL migrations。
- [ ] 创建索引和约束。
- [ ] 增加 migration test。

**完成标准：** 本地和云端 schema 可独立从空库初始化，并能从上一版本升级。

### Phase 2：Task Domain

- [ ] Task Repository。
- [ ] Project Repository。
- [ ] TaskDependency Repository。
- [ ] TaskService CRUD。
- [ ] 状态机校验。
- [ ] 子任务。
- [ ] 依赖环检测。
- [ ] 截止时间。
- [ ] priority/context。
- [ ] estimated/actual duration。
- [ ] 完成与重新打开。
- [ ] CompletionResult。
- [ ] 单元测试。
- [ ] Repository integration test。

**完成标准：** 不依赖 UI 即可通过领域服务完成完整一次性任务生命周期。

### Phase 3：Recurrence

- [ ] Recurrence parser/validator。
- [ ] 重复任务 occurrence 生成策略。
- [ ] 每日规则。
- [ ] 每周规则。
- [ ] 每月规则。
- [ ] interval。
- [ ] until/count。
- [ ] skip occurrence。
- [ ] edit one occurrence。
- [ ] edit future occurrences。
- [ ] 保留历史 occurrence。
- [ ] DST/时区单元测试。

**完成标准：** 完成一个 occurrence 不影响后续 occurrence，修改未来规则不破坏历史。

### Phase 4：Waiting Domain

- [ ] Waiting Repository。
- [ ] WaitingService CRUD。
- [ ] expected_at。
- [ ] follow_up_at。
- [ ] resolve/cancel。
- [ ] Task → Waiting 关联。
- [ ] Waiting → Task 转换。
- [ ] 幂等转换测试。
- [ ] 等待总览 query。

**完成标准：** WaitingItem 可独立使用，也可完整接入任务闭环。

### Phase 5：Calendar Domain

- [ ] Calendar Repository。
- [ ] CalendarService CRUD。
- [ ] 全天事件。
- [ ] 定时事件。
- [ ] 时区字段。
- [ ] 重复事件。
- [ ] Task schedule。
- [ ] 时间冲突检测。
- [ ] 月/周/日 range query。
- [ ] DST 测试。

**完成标准：** 日历和任务保持独立实体，同时可通过 schedule 关系联动。

### Phase 6：Memo + Reminder Domain

- [ ] Memo Repository。
- [ ] MemoService CRUD。
- [ ] 快速创建。
- [ ] 编辑。
- [ ] 置顶。
- [ ] 归档/恢复。
- [ ] 标签/上下文。
- [ ] plain_text 搜索字段。
- [ ] Reminder Repository。
- [ ] ReminderService。
- [ ] Task reminder。
- [ ] Event reminder。
- [ ] Waiting follow-up reminder。
- [ ] Memo reminder。
- [ ] Scheduler 重启恢复。
- [ ] fire_key 幂等。
- [ ] snooze/dismiss/cancel。
- [ ] Memo → Task。
- [ ] Memo → CalendarEvent。
- [ ] Memo → WaitingItem。
- [ ] 转换来源关联。
- [ ] 转换事务失败回滚。
- [ ] Memo 与 Task/Note 的边界测试。

**完成标准：** Memo 可以完全独立使用，也可以无损转入行动体系；四类实体共用一套提醒机制。

### Phase 7：Sync + Cloud

- [ ] 注册所有 EPIC-20 entity_type。
- [ ] 本地 outbox 接入。
- [ ] Cloud repository。
- [ ] Cloud sync mapping。
- [ ] push create/update/delete。
- [ ] pull create/update/delete。
- [ ] tombstone。
- [ ] duplicate change_id。
- [ ] base_version conflict。
- [ ] recurrence 冲突。
- [ ] Memo 文本冲突。
- [ ] 离线创建任务测试。
- [ ] 离线创建 Memo 测试。
- [ ] 离线完成任务测试。
- [ ] 恢复联网同步测试。
- [ ] 第二设备 pull 验证。

**完成标准：** Task/Event/Waiting/Memo 在设备 A 离线修改后，恢复网络可以同步到 Cloud 并被设备 B 拉取。

### Phase 8：Desktop UI

- [ ] 增加“执行”一级入口。
- [ ] Today Overview。
- [ ] Task List。
- [ ] Task Detail。
- [ ] Project filter。
- [ ] Subtask UI。
- [ ] Dependency UI。
- [ ] Recurrence editor。
- [ ] Calendar month/week/day。
- [ ] Waiting overview。
- [ ] Memo quick capture。
- [ ] Memo pinned/archive/search。
- [ ] Reminder editor。
- [ ] Memo convert actions。
- [ ] Task schedule action。
- [ ] 右键菜单。
- [ ] 空状态、加载状态、错误状态。
- [ ] 键盘可访问性。
- [ ] 统一 Toast/通知，不持续遮挡界面。

**完成标准：** 用户无需开发工具即可完成本 EPIC 的所有核心流程。

### Phase 9：关联、搜索、可观测性与收尾

- [ ] EPIC-13 relation 接入。
- [ ] Task ↔ Note/File/Email。
- [ ] Waiting ↔ Contact/Email。
- [ ] Memo ↔ Note/File。
- [ ] CompletionResult ↔ Note/File/Email。
- [ ] EPIC-14 搜索实体注册。
- [ ] 时间线事件注册。
- [ ] EPIC-19 structured logs。
- [ ] Audit Log。
- [ ] 性能测试。
- [ ] E2E 测试。
- [ ] 回归测试。
- [ ] 更新用户文档。
- [ ] 更新架构文档。

---

## 21. 推荐 PR / Commit 拆分

不要用一个超大 PR 完成本 EPIC。建议至少拆为：

1. `epic20-contracts-and-schema`
2. `epic20-task-domain`
3. `epic20-recurrence`
4. `epic20-waiting-domain`
5. `epic20-calendar-domain`
6. `epic20-memo-and-reminders`
7. `epic20-sync-and-cloud`
8. `epic20-desktop-execution-ui`
9. `epic20-relations-observability-e2e`

每个 PR 必须能独立构建，并包含对应测试。

---

## 22. 测试矩阵

### 22.1 单元测试

- Task 状态机。
- 子任务规则。
- dependency cycle。
- recurrence parser。
- occurrence generator。
- Waiting 状态机。
- Calendar range validation。
- timezone conversion。
- Memo archive/restore。
- Memo conversion。
- Reminder fire idempotency。

### 22.2 数据库测试

- 新库 migration。
- 老库 upgrade。
- rollback / transaction failure。
- indexes 存在。
- FK/unique/check constraint 生效。
- soft delete。

### 22.3 同步测试

至少使用两个逻辑设备：

```text
Device A offline create
→ reconnect
→ push
→ Cloud
→ Device B pull
→ compare entity
```

覆盖 Task、CalendarEvent、WaitingItem、Memo、Reminder。

### 22.4 冲突测试

- 两设备同时改 Task title。
- 一台 complete，另一台 cancel。
- 两设备修改 recurrence。
- 两设备同时修改 Memo content。
- 一台 archive Memo，另一台 edit Memo。
- 重复 push 同一个 change_id。

### 22.5 E2E 核心场景

#### 场景 A：普通任务

```text
创建任务
→ 设置截止时间
→ 设置提醒
→ 开始执行
→ 完成
→ 写结果总结
→ 关联 Note/File
```

#### 场景 B：等待回复

```text
创建 Task
→ 标记 waiting
→ 创建 WaitingItem
→ 设置 2 天后跟进
→ 收到结果
→ resolve WaitingItem
→ Task 回到 in_progress
→ complete
```

#### 场景 C：备忘录转行动

```text
快速创建 Memo
→ 置顶
→ 设置提醒
→ 后续决定需要处理
→ 转 Task
→ 原 Memo 归档
→ Task 保留 derived_from Memo 关系
```

#### 场景 D：日历安排

```text
Task
→ schedule to calendar
→ 检测时间冲突
→ 用户确认
→ CalendarEvent 创建
→ 移动 CalendarEvent
→ Task 仍保持原业务状态
```

#### 场景 E：离线同步

```text
断网
→ 创建 Memo + Task
→ 完成一个 Task
→ 重启应用
→ 数据仍在
→ 恢复网络
→ 自动同步
→ 第二设备看到相同结果
```

---

## 23. 性能与数据规模验证

正式验收前构造至少一个中等规模测试库，用于暴露 N+1 查询和全表扫描：

```text
Projects: 100
Tasks: 10,000+
Task occurrences: 20,000+
Calendar events: 10,000+
Waiting items: 2,000+
Memos: 10,000+
Reminders: 10,000+
Relations: 20,000+
```

重点测：

- Today Overview。
- Calendar month query。
- overdue tasks。
- waiting follow-up query。
- Memo 最近列表和搜索。
- reminders due scan。

如果查询出现明显线性退化，应先修索引/查询，不允许通过 UI 少显示数据掩盖问题。

---

## 24. 隐私与安全

- Memo、Task description、Waiting description 都属于个人敏感内容，日志不得记录全文。
- Cloud API 必须校验当前用户是否拥有实体。
- entity_id 不可作为授权依据。
- AI Tool 在 EPIC-22 接入前不得拥有绕过用户确认的写入口。
- 删除采用 soft delete + sync tombstone；真正物理清理遵循全局数据生命周期策略。
- Audit Log 只记录必要元数据，不复制业务正文。

---

## 25. 与后续 EPIC 的接口

### EPIC-21：统一领域服务与 AI 可操作接口

EPIC-20 必须提供稳定 Command/Query Service，使 EPIC-21 只做工具适配，不重新实现任务业务规则。

### EPIC-22：AI Tool Registry

后续工具可映射：

```text
create_task
complete_task
create_calendar_event
create_waiting_item
resolve_waiting_item
create_memo
set_reminder
convert_memo_to_task
```

工具写操作继续受确认、权限、审计和撤销机制约束。

### EPIC-26：统一输入与收件箱

EPIC-26 可以把输入路由到 Memo/Task/Event/Waiting，但 EPIC-20 不提前实现完整 Inbox。

### EPIC-27：邮件聚合与行动中心

邮件模块应调用本 EPIC 已完成的领域服务：

```text
Email → Task
Email → CalendarEvent
Email → WaitingItem
```

不得在邮件模块新建另一套“邮件任务表”。

---

## 26. Definition of Done

EPIC-20 只有同时满足以下条件才算完成：

### 功能

- [ ] 一次性任务完整生命周期可用。
- [ ] 重复任务完整生命周期可用。
- [ ] 子任务和前置依赖可用。
- [ ] WaitingItem 可独立创建、跟进、解决。
- [ ] WaitingItem 可转 Task。
- [ ] LifeTrace 内部日历支持全天/定时/重复事件。
- [ ] Task 可安排到 CalendarEvent。
- [ ] 时间冲突有明确提示。
- [ ] Reminder 为通用能力，不是模块私有逻辑。
- [ ] **Memo 可快速创建、编辑、置顶、提醒、归档、搜索。**
- [ ] **Memo 可转 Task / CalendarEvent / WaitingItem。**
- [ ] **Memo 转换后保留来源关系。**
- [ ] 完成结果可以追溯原 Task。
- [ ] Task/Event/Waiting/Memo 能关联其他 LifeTrace 实体。

### 本地优先与同步

- [ ] 断网可创建和编辑所有核心实体。
- [ ] 断网可完成任务、归档 Memo。
- [ ] 应用重启数据不丢失。
- [ ] 恢复联网后自动进入同步链路。
- [ ] 云端保存完整副本。
- [ ] 第二设备可以拉取变更。
- [ ] 重复提交不产生重复记录。
- [ ] 冲突有确定行为，不静默覆盖关键数据。

### 工程质量

- [ ] Rust/TypeScript contracts 对齐。
- [ ] SQLite/PostgreSQL schema 对齐。
- [ ] 所有 mutation 走 Domain Service。
- [ ] 所有 mutation 进入 Sync Outbox。
- [ ] 所有关键 mutation 有 Audit Log。
- [ ] 日志不泄露业务正文。
- [ ] 单元测试通过。
- [ ] migration test 通过。
- [ ] sync integration test 通过。
- [ ] E2E 核心场景通过。
- [ ] lint/typecheck/build 通过。

### 架构边界

- [ ] 没有在 EPIC-20 内重复实现 Notes。
- [ ] 没有提前实现邮件聚合。
- [ ] 没有提前实现 AI 自主执行。
- [ ] 没有创建第二套 entity relation。
- [ ] 没有为 Task/Event/Waiting/Memo 分别复制提醒系统。

---

## 27. Agent 开发时的硬性约束

后续让 Agent 执行 EPIC-20 时，应附带以下要求：

1. **先审计现有实现再改代码，不根据目录名猜架构。**
2. **每完成一个 Phase 先跑对应测试，再进入下一 Phase。**
3. **不得直接删除或绕过现有同步、日志、关联机制。**
4. **不得为了 UI 方便复制领域状态。** UI store 只能缓存展示状态，数据库/Domain 才是事实来源。
5. **所有时间字段先明确语义后编码。** 不允许全部使用字符串然后靠前端猜时区。
6. **所有转换操作使用事务并保证幂等。** 特别是 Memo → Task/Event/Waiting。
7. **重复任务历史不可因为修改 recurrence 被重写。**
8. **提醒必须持久化。** 不能只使用进程内 `setTimeout`。
9. **错误不能只包装成“操作失败”。** 必须保留结构化 error_code，并接入 EPIC-19 日志。
10. **每个新表和高频查询必须评估索引。**
11. **每个新同步实体必须同时验证 push、pull、delete、conflict、idempotency。**
12. **不记录 Memo/Task/Email/Note 正文到日志。**
13. **不得把 Memo 实现为 Task 的特殊状态。**
14. **不得把 Memo 实现为 Notes 富文本记录的别名。**
15. **不得为备忘录单独复制一套提醒基础设施。**

---

## 28. 最终交付物

完成 EPIC-20 后至少应产生：

```text
1. EPIC-20 schema / migration
2. contracts DTO / enums / error codes
3. Task domain + repository
4. Recurrence engine
5. Waiting domain + repository
6. Calendar domain + repository
7. Memo domain + repository
8. Shared Reminder service + scheduler
9. CompletionResult
10. Relation integration
11. Sync client integration
12. Cloud PostgreSQL + sync handling
13. Desktop Today / Tasks / Calendar / Waiting / Memo UI
14. Observability + audit
15. Unit / integration / sync / E2E tests
16. Developer documentation
17. User-facing behavior documentation
```

EPIC-20 完成后，LifeTrace 才具备稳定的“记录 → 安排 → 执行 → 等待 → 跟进 → 完成 → 结果追溯”基础闭环，并为后续邮件行动中心和 AI 管家提供可靠的可操作领域能力。
