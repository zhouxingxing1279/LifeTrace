# LifeTrace 后续功能与架构演进需求

> 用途：记录 LifeTrace 后续功能、Android 客户端、自动记账、账单对账、云端同步和数据架构改造需求。  
> 状态：规划中  
> 原则：本地优先、离线可用、云端完整副本、渐进式重构。

---

## 1. 项目目标

LifeTrace 后续需要从单机 Windows 应用演进为：

```text
Windows 客户端
├── 本地 SQLite
├── 本地文件
└── 同步客户端
          ↕ HTTPS
LifeTrace Cloud
├── 用户与设备
├── 完整业务数据
├── 增量同步服务
├── 变更日志
├── 文件对象存储
└── 备份
          ↕ HTTPS
Android 客户端
├── 本地 SQLite
├── 快速记账
├── 通知辅助识别
└── 同步客户端
```

最终实现：

- 手机、电脑和云端各保存一份完整数据。
- 手机和电脑离线时仍可使用。
- 联网后自动增量同步。
- Android 端重点优化快速记账。
- 电脑端重点负责账单导入、整理、对账和分析。
- 云端负责完整数据副本、设备同步和灾难恢复。
- 逐步连接财务、习惯、训练、笔记、英语、照片、健康和复盘数据。

---

## 2. 总体设计原则

### 2.1 本地优先

- 所有用户操作先写入本地 SQLite。
- 网络不可用时仍能记账、打卡和编辑。
- 同步失败不能阻塞前台业务。
- 云端不可用时本地数据仍完整可用。

### 2.2 云端完整副本

云端不是简单中转站，需要保存完整、可查询、可恢复的业务数据。

同一条记录在手机、电脑和云端使用相同的全局 ID：

```text
entity_id
version
updated_at
modified_by_device
deleted_at
```

### 2.3 增量同步

禁止直接同步或覆盖整个 SQLite 文件。

同步内容应为：

- 新增记录
- 修改记录
- 删除墓碑
- 文件元数据
- 同步游标
- 冲突状态

### 2.4 渐进式开发

优先顺序：

```text
财务数据重构
→ Android 快速记账
→ 财务三端同步
→ 习惯与复盘同步
→ 笔记同步
→ 照片、英语和健康
```

---

# 3. 数据层重构

## 3.1 当前问题

部分业务表采用：

```sql
id TEXT PRIMARY KEY,
data_json TEXT NOT NULL,
updated_at TEXT NOT NULL
```

该方案适合快速迁移，但不利于：

- 金额统计
- 时间范围查询
- 排序
- 普通索引
- 外键约束
- 唯一约束
- 数据类型校验
- 多端同步
- 字段级冲突判断

## 3.2 应改成真实列的数据

满足以下任意条件时，应优先使用真实列：

- 经常用于 `WHERE`
- 经常用于 `ORDER BY`
- 经常用于 `GROUP BY`
- 需要建立索引
- 需要唯一约束
- 需要外键
- 属于金额、日期、状态、ID
- 决定记录生命周期

典型字段：

```text
id
user_id
account_id
category_id
activity_id
amount_cents
occurred_at
status
version
created_at
updated_at
deleted_at
```

## 3.3 可以继续使用 JSON 的数据

- Tiptap 富文本结构
- AI 消息数组
- 第三方接口原始响应
- 账单导入原始字段
- OCR 原始解析结果
- 显示配置
- 复杂提醒规则
- 照片 EXIF 扩展信息
- 低频 metadata

## 3.4 重构优先级

### 第一阶段：财务

- `transactions`
- `finance_accounts`
- `categories`
- `transaction_evidence`
- `import_batches`

要求：

- 金额使用整数分 `amount_cents`。
- 交易时间使用明确时区。
- 支持支出、收入、转账、退款和手续费。
- 支持外部交易号唯一约束。
- 支持临时账和正式账状态。

### 第二阶段：习惯与复盘

- `activities`
- `activity_logs`
- `daily_reviews`

要求：

- 本地自然日使用 `YYYY-MM-DD`。
- 一日一条复盘可通过唯一约束保证。
- 打卡记录支持日期范围统计。
- 习惯与打卡建立外键关系。

### 第三阶段：训练和英语

- `workouts`
- `workout_exercises`
- `workout_sets`
- `english_learning_records`
- `english_vocabulary`

复杂原始数据可继续保留 `raw_json`。

### 第四阶段：笔记元数据

拆出：

- 标题
- 文件夹 ID
- 收藏、置顶、归档和删除状态
- 创建时间和更新时间
- 纯文本索引内容
- 版本号

继续保留：

- `content_json`
- `content_html`
- `content_markdown`

---

# 4. Android 客户端

## 4.1 产品定位

第一版不复制完整桌面端，优先作为：

> LifeTrace 移动记账伴侣和快速记录入口。

## 4.2 第一版功能

- 快速支出
- 快速收入
- 账户间转账
- 账户和分类
- 常用模板
- 最近账单
- 待确认账单箱
- 本地 SQLite
- 后台同步
- Android 分享入口
- 支付通知辅助识别
- 桌面小组件
- 快捷设置图块
- 长按应用图标快捷入口

## 4.3 快速记账页面

```text
¥ 35.00

支出｜收入｜转账

餐饮  交通  购物  学习  更多

微信  支付宝  银行卡  现金

保存
```

要求：

- 默认当前时间。
- 默认最近使用账户。
- 分类按使用频率排序。
- 商户和备注可选。
- 保存后快速返回。
- 支持撤销。
- 普通消费在 2～3 次操作内完成。

## 4.4 一键记账方式

### 通知栏一键分类

```text
检测到微信支付 ¥35.00

[餐饮] [交通] [购物] [忽略]
```

用户点击一次分类后保存为临时账单。

### 高置信度零点击暂记

```text
已自动记录：
¥18.00 · 瑞幸咖啡 · 餐饮

[撤销] [修改]
```

记录状态先设为：

```text
provisional
```

正式账单对账后改为：

```text
confirmed
```

### 桌面小组件

提供：

- 本月支出
- 快速支出
- 快速收入
- 常用固定模板

### 快捷设置图块

Android 下拉快捷设置中增加：

```text
LifeTrace 记账
```

### 应用快捷方式

长按图标显示：

- 记支出
- 记收入
- 记转账
- 扫描票据

---

# 5. 自动记账与候选交易

## 5.1 数据来源

候选交易可能来自：

- 手工记账
- 微信通知
- 支付宝通知
- 银行 App 通知
- 可选无障碍页面识别
- 支付截图
- 小票 OCR
- 微信账单导入
- 支付宝账单导入
- 银行流水导入
- Android 分享入口

## 5.2 候选交易模型

```ts
interface TransactionCandidate {
  id: string;
  sourceType:
    | "manual"
    | "notification"
    | "accessibility"
    | "bill_import"
    | "ocr"
    | "share";
  sourceApp?: string;
  direction:
    | "expense"
    | "income"
    | "transfer"
    | "refund"
    | "unknown";
  amountCents?: number;
  merchant?: string;
  accountHint?: string;
  occurredAt: string;
  confidence: number;
  status:
    | "candidate"
    | "provisional"
    | "confirmed"
    | "ignored";
}
```

## 5.3 通知监听

读取：

- packageName
- title
- text
- bigText
- textLines
- subText
- postTime
- channelId
- notification key
- notification id
- tag
- group key

优先监听：

- 微信
- 支付宝
- 常用银行 App
- 云闪付

## 5.4 微信特殊情况

需要区分：

- 微信支付消费
- 微信收款
- 好友转账
- 转账待收款
- 转账已收款
- 转账退还
- 领取红包
- 发送红包
- 红包退回
- 退款

微信红包通知没有金额时，只创建候选线索：

```text
type = wechat_red_packet
amount = null
status = waiting_for_bill
```

后续由微信正式账单补全。

## 5.5 支付宝特殊情况

覆盖：

- 支付宝余额
- 余额宝
- 花呗
- 银行卡快捷支付
- 转账
- 收款
- 退款

通知缺失时依赖：

- 分享截图
- 可选无障碍识别
- 正式账单导入

## 5.6 无障碍服务

作为实验性可选功能，默认关闭。

要求：

- 只处理支付应用白名单。
- 只在支付成功页面提取必要字段。
- 不保存无关页面内容。
- 不上传完整页面文本。
- 明确解释权限用途。
- 不能作为唯一记账依据。

---

# 6. 手工记账与正式账单去重

## 6.1 核心原则

导入账单时禁止直接全部新增。

正确流程：

```text
解析账单
→ 检查外部交易号
→ 查找已有交易
→ 计算匹配分数
→ 自动合并或人工确认
→ 无匹配才新增
```

## 6.2 交易和证据分离

```text
Transaction
└── 一笔真实交易

TransactionEvidence
├── 手工记录
├── 微信通知
├── 支付宝通知
├── 银行通知
├── 微信正式账单
├── 支付宝正式账单
└── 银行流水
```

## 6.3 交易状态

```text
draft
provisional
confirmed
needs_review
deleted
```

## 6.4 匹配优先级

### 第一层：外部交易号

使用：

```text
source + external_transaction_id
```

建立唯一约束，防止同一账单重复导入。

### 第二层：强匹配

- 金额完全一致
- 收支方向一致
- 时间接近
- 支付渠道相同或兼容
- 原记录未被其他正式账单确认

### 第三层：评分匹配

参考评分：

| 条件 | 分值 |
|---|---:|
| 金额完全相同 | +50 |
| 时间差不超过 2 分钟 | +25 |
| 时间差不超过 10 分钟 | +15 |
| 支付渠道相同 | +15 |
| 资金账户相同 | +15 |
| 商户名称相似 | +15 |
| 收支方向一致 | +10 |
| 分类相同 | +5 |
| 已匹配正式账单 | -100 |
| 时间相差超过一天 | -100 |

建议：

```text
≥ 85：自动合并
60～84：进入待确认
< 60：创建新交易
```

实际阈值通过真实账单样本调整。

## 6.5 字段合并规则

正式账单优先补充：

- 外部交易号
- 精确金额
- 精确时间
- 商户
- 支付渠道
- 资金账户

用户输入优先保留：

- 分类
- 备注
- 标签
- 项目关联
- 自定义描述

原则：

```text
正式账单补充客观事实
用户输入保留个人语义
```

## 6.6 特殊交易

### 转账

账户间转账不计为支出和收入：

```text
from_account_id
to_account_id
amount_cents
fee_cents
```

### 退款

退款保留独立流水，并关联：

```text
refund_of_transaction_id
```

### 手续费

手续费单独记录，不能混入转账本金。

### 红包

区分：

- 领取
- 发送
- 退回
- 群红包退款
- 无金额通知线索

---

# 7. 独立云端同步服务

## 7.1 服务定位

新增独立项目：

```text
lifetrace-sync-server
```

它与现有桌面本地服务分离，不直接暴露桌面端 `127.0.0.1` 服务。

## 7.2 推荐技术栈

- Rust
- Axum
- PostgreSQL
- SQLx
- Docker Compose
- Caddy
- JWT 或 Session Token
- Argon2id
- S3 兼容对象存储

## 7.3 推荐仓库结构

```text
LifeTrace/
├── crates/
│   ├── lifetrace-core/
│   ├── lifetrace-models/
│   └── lifetrace-sync-protocol/
├── src-tauri/
├── android/
└── sync-server/
```

共享内容：

- 数据模型
- 同步协议
- 金额处理
- 字段校验
- 账单匹配
- 重复检测
- 版本规则

---

# 8. 客户端同步机制

## 8.1 `sync_outbox`

```sql
CREATE TABLE sync_outbox (
    change_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    base_version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);
```

业务写入和 outbox 写入必须在同一个数据库事务中。

## 8.2 `sync_state`

```sql
CREATE TABLE sync_state (
    scope TEXT PRIMARY KEY,
    cursor INTEGER NOT NULL,
    last_synced_at TEXT
);
```

## 8.3 `sync_conflicts`

```sql
CREATE TABLE sync_conflicts (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    local_payload_json TEXT NOT NULL,
    remote_payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    resolved_at TEXT
);
```

## 8.4 同步流程

手机新增账单：

```text
写入手机 SQLite
→ 同事务写入 sync_outbox
→ 页面立即显示
→ 后台 push 到云端
→ 云端保存完整交易
→ 云端产生变更游标
→ 电脑 pull
→ 写入电脑 SQLite
```

电脑导入账单并补全交易后，反向同步到手机。

---

# 9. 云端同步接口

## 9.1 认证

```http
POST /api/auth/login
POST /api/auth/refresh
POST /api/auth/logout
```

## 9.2 设备

```http
POST   /api/devices/register
GET    /api/devices
DELETE /api/devices/{id}
```

## 9.3 推送

```http
POST /api/sync/push
```

每个变更包含：

```text
change_id
device_id
entity_type
entity_id
operation
base_version
payload
```

## 9.4 拉取

```http
GET /api/sync/pull?cursor={cursor}&limit={limit}
```

## 9.5 全量快照

```http
GET /api/sync/snapshot
```

用于：

- 新手机
- 新电脑
- 本地数据库损坏
- 灾难恢复

## 9.6 幂等性

`change_id` 必须唯一。

重复上传同一个变更时，服务器只处理一次。

---

# 10. 云端数据库

## 10.1 基础表

- users
- devices
- refresh_tokens
- sync_changes
- sync_requests
- files
- import_batches

以及完整业务表：

- transactions
- transaction_evidence
- finance_accounts
- categories
- activities
- activity_logs
- daily_reviews
- notes
- note_folders
- note_tags
- workouts
- english records

## 10.2 变更日志

```sql
CREATE TABLE sync_changes (
    cursor BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    device_id UUID NOT NULL,
    change_id UUID NOT NULL UNIQUE,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    operation TEXT NOT NULL,
    version BIGINT NOT NULL,
    payload JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## 10.3 全局 ID

使用客户端生成的 UUID 或 UUIDv7。

禁止跨设备使用自增整数作为业务主键。

## 10.4 版本控制

每条记录包含：

```text
version
modified_by_device
updated_at
deleted_at
```

客户端提交：

```text
base_version
```

若云端版本已变化，则返回冲突。

## 10.5 删除墓碑

同步记录不得立即物理删除。

使用：

```text
deleted_at
version + 1
```

墓碑同步到所有设备后，经过保留期再清理。

---

# 11. 冲突处理

| 模块 | 推荐策略 |
|---|---|
| 账单 | 版本校验，必要时人工选择 |
| 账户 | 最后修改优先并记录冲突 |
| 分类 | 最后修改优先 |
| 习惯 | 版本校验 |
| 打卡 | 按记录 ID 合并 |
| 每日复盘 | 同日冲突保留两版 |
| 笔记 | 生成冲突副本 |
| 设置 | 最后修改优先 |
| AI 对话 | 按消息 ID 追加 |
| 照片 | SHA-256 去重 |
| 附件 | SHA-256 与元数据同步 |

第一版不做复杂字段级自动合并。

---

# 12. 文件和照片同步

## 12.1 存储方式

PostgreSQL 保存：

- 文件 ID
- 所属用户
- SHA-256
- storage key
- 原始文件名
- MIME
- 大小
- 创建时间

文件本体保存到：

- Cloudflare R2
- MinIO
- 阿里云 OSS
- 腾讯云 COS
- 其他 S3 兼容存储

## 12.2 文件同步流程

```text
计算 SHA-256
→ 查询云端是否存在
→ 获取上传地址
→ 上传对象存储
→ 同步文件元数据
→ 其他设备按需下载
```

## 12.3 照片策略

- 先同步元数据。
- 同步缩略图。
- 原图按需下载。
- 避免新设备首次登录下载全部照片。
- 使用文件哈希去重。

---

# 13. 账号、设备和安全

## 13.1 认证

- 密码使用 Argon2id。
- Access Token 短期有效。
- Refresh Token 可轮换和撤销。
- 浏览器使用 Secure、HttpOnly Cookie。
- Tauri 和 Android 将 Refresh Token 保存到安全凭据。
- 不把 Token 和 API Key 保存为普通业务 JSON。

## 13.2 设备管理

显示：

- 设备名
- 平台
- 最后同步时间
- 最近 IP
- 状态
- 撤销按钮

手机丢失后可以撤销设备。

## 13.3 网络安全

- 所有通信使用 HTTPS。
- 数据库端口不得暴露公网。
- 服务器只公开 80 和 443。
- 上传接口限制大小和 MIME。
- 日志不得记录账单正文、密码、Token 和 AI Key。
- 文件下载必须校验所属用户。

## 13.4 敏感数据

默认不跨设备同步：

- DeepSeek API Key
- 本地照片证书私钥
- Windows 凭据
- 临时文件路径
- 本机环境配置

---

# 14. 云服务器部署

## 14.1 推荐规格

```text
2 vCPU
4 GB RAM
60～90 GB SSD
Ubuntu 24.04 LTS
```

## 14.2 部署组件

```text
Caddy
Rust/Axum Sync Server
PostgreSQL
Docker Compose
定时备份
S3 兼容对象存储
```

## 14.3 部署结构

```text
HTTPS
  ↓
Caddy
  ↓
LifeTrace Sync Server
  ↓
PostgreSQL
  ↓
对象存储
```

## 14.4 备份

云端同步副本不等于备份。

必须实现：

- 每日数据库备份
- 每周完整备份
- 7～30 天历史保留
- 文件清单和校验和
- 备份加密
- 恢复测试
- 独立位置保存备份

---

# 15. 其他后续功能

## 15.1 周报和月报

自动汇总：

- 习惯完成率
- 连续坚持天数
- 训练次数和趋势
- 支出和分类变化
- 异常消费
- 英语学习情况
- 新建笔记
- 情绪和精力变化
- 下周期建议

## 15.2 统一生活时间线

合并展示：

- 打卡
- 训练
- 财务
- 笔记
- 英语
- 复盘
- 照片
- 目标进度

## 15.3 全局搜索

搜索：

- 笔记正文
- 账单商户
- 分类
- 习惯
- 训练动作
- 英语文章
- 生词
- 复盘
- 文件名
- 照片元数据

## 15.4 目标和里程碑

```text
Goal
├── Milestone
├── Activity
├── Task
├── Note
└── Review
```

## 15.5 待办收件箱

临时内容可转换为：

- 待办
- 习惯
- 日历事件
- 笔记
- 目标里程碑

## 15.6 智能提醒

- 固定时间提醒
- 条件提醒
- 连续未完成提醒
- 预算超限提醒
- 还款提醒
- 单词复习提醒
- 项目长期无进展提醒

## 15.7 财务预算和订阅

- 总预算
- 分类预算
- 超支提醒
- 固定支出
- 月度订阅
- 年度订阅
- 即将续费
- 长期未使用订阅

## 15.8 健身分析

- 动作重量趋势
- 估算 1RM
- 训练容量
- 肌群频率
- 个人纪录
- 计划执行率
- 疲劳和恢复评分

## 15.9 健康档案

- 身高、体重、腰围
- 血压、心率
- 体检报告
- 检验指标
- 药物
- 过敏
- 既往病史
- 就诊记录

健康数据默认不向 AI 开放，必须单独授权。

## 15.10 数据关联

支持笔记、账单、训练、习惯、复盘、目标和照片之间的关联。

## 15.11 数据健康中心

显示：

- 数据库状态
- 最近备份
- 同步状态
- 冲突数量
- 失败任务
- 孤立附件
- 重复文件
- 照片和附件占用
- 墓碑数量

## 15.12 完整备份

```text
LifeTraceBackup.zip
├── manifest.json
├── database.json 或 database.sqlite
├── attachments/
├── photos/
├── checksum.json
└── version.json
```

支持：

- 数据库备份
- 完整备份
- 增量备份
- 定时备份
- 恢复预览
- 旧版本兼容

---

# 16. AI 管家演进

## 16.1 数据权限中心

按数据集授权：

- 财务
- 习惯
- 训练
- 笔记
- 英语
- 健康
- 照片元数据

## 16.2 建议—预览—确认

```text
用户提出需求
→ AI 生成操作预览
→ 用户确认
→ 正式写入
```

逐步支持：

- 创建待办
- 创建习惯
- 创建目标
- 生成周报
- 整理复盘
- 生成预算建议
- 从笔记提取行动项
- 从账单识别订阅
- 从训练数据生成计划
- 从体检报告提取指标

## 16.3 安全要求

- AI 默认只读。
- 写操作必须确认。
- 写操作支持撤销。
- 未授权数据不得发送到第三方。
- API Key 使用安全凭据存储。
- 提供数据发送预览。

---

# 17. 推荐开发路线

## Phase 0：基础稳定性

- [ ] 修复本地日期和 UTC 日期混用
- [ ] 收紧本地 API CORS
- [ ] 增加本地 API 会话保护
- [ ] AI Key 安全存储
- [ ] 附件大小和类型限制
- [ ] 备份完整性验证
- [ ] 建立数据库迁移框架

## Phase 1：财务重构

- [ ] 账户表规范化
- [ ] 交易表规范化
- [ ] 分类表
- [ ] 交易证据表
- [ ] 导入批次表
- [ ] 金额整数化
- [ ] 账单去重
- [ ] 正式账单对账
- [ ] 转账和退款模型

## Phase 2：Android 快速记账

- [ ] Android 应用框架
- [ ] 快速支出、收入和转账
- [ ] 本地 SQLite
- [ ] 常用模板
- [ ] 最近账单
- [ ] 待确认箱
- [ ] 通知监听
- [ ] 一键分类通知
- [ ] 分享入口
- [ ] 桌面小组件
- [ ] 快捷设置图块

## Phase 3：财务云同步 0.1

- [ ] 独立同步协议
- [ ] 独立同步服务器
- [ ] 用户登录
- [ ] 设备注册和撤销
- [ ] sync_outbox
- [ ] push
- [ ] pull
- [ ] snapshot
- [ ] cursor
- [ ] version
- [ ] tombstone
- [ ] 冲突检测
- [ ] HTTPS
- [ ] 云端每日备份

## Phase 4：习惯和复盘

- [ ] 习惯同步
- [ ] 打卡同步
- [ ] 每日复盘同步
- [ ] 目标和里程碑
- [ ] 条件提醒

## Phase 5：笔记与附件

- [ ] 文件夹和标签同步
- [ ] 笔记内容同步
- [ ] 版本历史
- [ ] 冲突副本
- [ ] 附件对象存储
- [ ] 按需下载

## Phase 6：分析能力

- [ ] 周报
- [ ] 月报
- [ ] 统一时间线
- [ ] 全局搜索
- [ ] 财务预算
- [ ] 订阅管理
- [ ] 健身趋势
- [ ] 数据健康中心

## Phase 7：照片、英语和健康

- [ ] 照片元数据同步
- [ ] 缩略图同步
- [ ] 原图按需下载
- [ ] 英语数据同步
- [ ] 健康档案
- [ ] 体检报告解析
- [ ] AI 权限中心

---

# 18. 最小可行版本

## LifeTrace Mobile + Sync 0.1

必须完成：

- Android 快速手工记账
- 支出、收入和转账
- Android 本地 SQLite
- 账户和分类
- 通知候选交易
- 待确认箱
- 微信账单导入
- 手工账和正式账单自动匹配
- Windows 和 Android 登录
- 独立云端同步服务
- 手机、电脑和云端各保存一份完整财务记录
- push / pull / snapshot
- outbox / cursor / version / tombstone
- 设备撤销
- HTTPS
- 云端备份

暂不包含：

- 全部 LifeTrace 模块同步
- 笔记实时协同
- 全部照片自动下载
- 多用户共享账本
- 端到端加密
- 短信自动记账
- Xposed 或 Root Hook
- 复杂机器学习分类
- 多节点高可用

---

# 19. 验收标准

## 19.1 记账

- [ ] Android 普通手工记账可在约 3 秒完成
- [ ] 固定模板支持一次点击记账
- [ ] 通知识别后可一次点击分类
- [ ] 离线时可正常记账
- [ ] 导入正式账单不会大量重复

## 19.2 对账

- [ ] 同一账单文件重复导入不重复创建交易
- [ ] 手工账可与正式账单自动匹配
- [ ] 匹配后保留用户分类和备注
- [ ] 模糊匹配进入待确认箱
- [ ] 转账、退款、手续费和红包正确处理

## 19.3 同步

- [ ] 手机新增账单后电脑可拉取
- [ ] 电脑补全正式账单后手机可更新
- [ ] 网络中断后可重试
- [ ] 重复 change_id 不重复写入
- [ ] 删除记录不会被离线设备复活
- [ ] 同时修改可检测冲突
- [ ] 新设备可通过 snapshot 恢复

## 19.4 云端

- [ ] 云端保存完整业务副本
- [ ] 数据库端口不暴露公网
- [ ] 所有业务接口要求认证
- [ ] 所有通信使用 HTTPS
- [ ] 设备可撤销
- [ ] 每日备份可验证和恢复

## 19.5 文件

- [ ] SHA-256 去重
- [ ] 上传失败不破坏业务记录
- [ ] 照片和附件按需下载
- [ ] 用户只能访问自己的文件
- [ ] 元数据和文件本体一致

---

# 20. 风险

## 通知不可靠

微信、支付宝和银行 App 不保证每笔交易都产生完整通知。通知只能作为实时线索，正式账单是最终对账依据。

## 无障碍权限敏感

必须默认关闭、白名单处理、最小采集，并明确告知用户。

## 同步复杂度

第一版只同步财务，避免同时处理笔记、照片和复杂冲突。

## 云端不等于备份

删除和错误修改也会同步，因此必须保留独立历史备份。

## 数据重构风险

采用渐进式迁移，并尽量保持现有前端 API 契约稳定。

---

# 21. GitHub Epic 建议

- [ ] Data Layer Refactor
- [ ] Android Quick Bookkeeping
- [ ] Transaction Reconciliation
- [ ] LifeTrace Cloud Sync
- [ ] File and Photo Sync
- [ ] Weekly and Monthly Insights
- [ ] Global Search and Timeline
- [ ] AI Permission and Action System

---

# 22. 最终愿景

LifeTrace 最终应成为：

```text
一个本地优先、云端有完整副本、
支持 Windows 与 Android 离线使用、
能够自动采集并核对个人数据、
连接财务、习惯、训练、笔记、英语、
健康、照片和复盘的个人管理系统。
```

核心能力：

```text
快速记录
自动补全
正式对账
三端同步
完整备份
数据关联
周期分析
智能提醒
AI 辅助
隐私可控
```
