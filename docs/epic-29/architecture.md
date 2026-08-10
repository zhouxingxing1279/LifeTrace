# EPIC-29 购物订单公共架构

> 日期：2026-08-10  
> 状态：公共核心已设计，平台 PoC 尚未接入

## 1. 架构目标

LifeTrace 的购物能力最终需要把不同平台的订单、履约、物流和退款信息统一到一个 Order Center 中，同时避免让淘宝、京东、拼多多、美团的页面结构和登录方式污染上层业务。

公共架构只解决一个问题：

> 无论底层通过 Windows WebView2、云端 Chromium、Browser XHR、DOM 解析还是未来官方 API 获取数据，上层都只接收统一订单模型和统一同步状态。

整体结构：

```text
                         LifeTrace Order Center
                                  │
                                  ▼
                         UnifiedOrder Model
                                  │
                                  ▼
                         Shopping Sync Engine
                                  │
             ┌────────────────────┼────────────────────┐
             │                    │                    │
             ▼                    ▼                    ▼
        TaobaoAdapter          JDAdapter           PddAdapter ...
             │                    │                    │
             ▼                    ▼                    ▼
        ShoppingSource       ShoppingSource       ShoppingSource
             │                    │                    │
      ┌──────┴──────┐       ┌─────┴─────┐       ┌─────┴─────┐
      │             │       │           │       │           │
 Windows WebView  Cloud   Browser XHR  DOM    Official API  ...
                 Browser
```

当前提交只实现中间公共层，不包含图中任何真实平台 Adapter。

---

## 2. 分层职责

### 2.1 ShoppingSource

`ShoppingSource` 是最低层采集边界。

它负责：

- 访问一个已认证的平台上下文
- 检查当前连接 / 登录状态
- 获取平台原始订单页或原始接口响应
- 可选获取原始订单详情
- 可选获取原始履约 / 物流详情

它不负责：

- 生成 LifeTrace 业务实体
- 写 SQLite / PostgreSQL
- 去重
- 财务匹配
- UI
- Cloud Sync

Source 可以有多种实现：

```text
WebView2Source
PlaywrightSource
BrowserXhrSource
OfficialApiSource
```

### 2.2 ShoppingAdapter

Adapter 是平台专用语义层。

它负责：

- 平台登录状态解释
- 平台分页方式
- 平台原始字段解析
- 平台订单状态映射
- 平台物流状态映射
- 平台退款状态映射
- 转换为 `UnifiedOrder`

例如后续：

```text
JDAdapter
├── JD Source
├── 京东订单字段映射
├── 京东履约字段映射
└── UnifiedOrder
```

而淘宝可以使用完全不同的原始数据结构，但最终输出相同模型。

### 2.3 Shopping Sync Engine

Sync Engine 不知道任何平台 URL、Cookie、Selector 或 API 参数。

它只负责：

```text
检查连接
→ 获取一页
→ 标准化
→ 校验
→ 去重
→ 输出 batch
→ 翻下一页
→ 到达边界停止
```

因此平台改版时，只需要修复对应 Adapter / Source。

---

## 3. 统一订单模型

核心模型：

```text
UnifiedOrder
├── platform
├── platformOrderId
├── 时间
├── 商户
├── 金额
├── OrderItem[]
├── Fulfillment[]
└── Refund[]
```

### 3.1 金额

金额统一使用最小货币单位整数，例如人民币分：

```text
¥399.00
↓
39900
```

公共校验会拒绝浮点金额进入统一模型。

### 3.2 OrderItem

支持一单多商品：

```text
Order
├── 键盘 × 1
├── 数据线 × 2
└── 鼠标垫 × 1
```

商品层可以保留平台商品 ID、SKU、图片和商品链接，但这些字段不是所有平台的强制字段。

---

## 4. 履约模型而不是单纯快递模型

统一模型使用 `Fulfillment`，而不是只使用 `Express` / `Shipment`。

原因是 LifeTrace 需要同时容纳：

```text
淘宝普通快递
京东自营配送
美团外卖即时配送
到店自取
虚拟商品
```

当前履约类型：

```text
parcel
platform_delivery
local_delivery
pickup
virtual
none
```

一个订单可以包含多个履约记录，例如拆单发货。

### TrackingEvent

物流 / 配送动态统一为：

```text
occurredAt
status
description
location?
```

因此未来 Order Center 可以统一生成：

```text
10:20 已发货
14:35 到达转运中心
次日 09:10 正在派送
11:42 已签收
```

而不需要关心数据来自京东订单页还是第三方物流 Provider。

---

## 5. 去重规则

第一层稳定业务键：

```text
platform + platformOrderId
```

例如：

```text
jd::123456

taobao::123456
```

两个平台即使出现相同订单号也不会冲突。

数据库层后续应建立等价唯一约束。

同步运行内部也会去除同一轮分页中重复出现的订单。

---

## 6. 增量同步

公共层区分两种 Cursor。

### 临时分页 Cursor

`pageToken` 只用于本次同步过程：

```text
page 1
→ pageToken A
→ page 2
→ pageToken B
→ page 3
```

同步结束后不要求长期保存它。

### 持久化同步 Cursor

`ShoppingSyncCursor` 表示下一次增量同步的水位，例如：

```text
latestOrderId
latestOrderTime
sourceData
```

平台 Adapter 可以使用 `sourceData` 保存少量平台专用水位，但上层不解释其意义。

### 已知订单边界

调用方还可以把数据库已经存在的订单 key 作为边界传入。

例如：

```text
本次页面

订单 D 新
订单 C 新
订单 B 已存在 ← stop
订单 A 更旧
```

Sync Engine 输出 D、C，然后立即停止，不再继续访问旧页面。

---

## 7. 同步状态机

```text
idle
  ↓
checking_connection
  ↓
syncing
  ├──────────────→ completed
  │
  ├─ 登录失效 ──→ paused(auth_required)
  │
  ├─ 用户验证 ──→ verification_required
  │
  ├─ 用户取消 ──→ paused(cancelled)
  │
  └─ 真实错误 ──→ failed
```

### 验证不是普通失败

以下情况统一进入需要用户处理的状态：

```text
slider
sms
qr
security_confirmation
login
unknown
```

公共层不实现验证码求解，也不会自动高频重试。

---

## 8. Verification Relay 未来接入

如果 PoC 最终选择云端 Browser Worker，验证流程可在不修改 Sync Engine 的情况下接入：

```text
Cloud Browser Worker
       │
       │ 遇到验证
       ▼
ShoppingError(VERIFICATION_REQUIRED)
       │
       ▼
Sync Engine
       │
       ▼
verification_required
       │
       ▼
Verification Relay
       │
       ▼
LifeTrace 手机端通知
       │
       ▼
用户远程操作原浏览器 Session
       │
       ▼
验证完成
       │
       ▼
重新启动一次增量同步
```

关键原则：

- 不自动求解滑块
- 不把验证答案拆出来转发
- 用户操作原认证上下文
- 验证完成后仍复用同一浏览器 Profile
- 远程验证入口必须独立做认证、短时有效和审计

Verification Relay 属于后续实现，不在当前公共核心中。

---

## 9. Windows Collector 与 Cloud Collector

公共架构不绑定采集发生的位置。

### Windows Collector

```text
LifeTrace Desktop
→ WebView2 / Chromium
→ Platform Source
→ Adapter
→ Sync Engine
```

优点：

- 用户本地完成登录和验证方便
- 本地网络环境更接近日常使用

限制：

- Windows 关机时无法持续更新

### Cloud Collector

```text
LifeTrace Cloud
→ Persistent Chromium Worker
→ Platform Source
→ Adapter
→ Sync Engine compatible contract
```

优点：

- Windows 关机后仍可定时更新
- 更适合统一订单动态中心

额外要求：

- 浏览器 Profile 属于高敏认证凭据
- 必须与普通业务数据库隔离
- 不记录 Cookie / Token 到日志
- 浏览器控制接口不得直接暴露公网
- 出现人机验证时暂停并交给用户

### Hybrid

最终也允许：

```text
Cloud Collector 作为主采集器
Windows Collector 作为 fallback / 调试入口
```

两者只要输出同一个 `UnifiedOrder`，上层无需区分来源。

---

## 10. 错误模型

统一错误码：

```text
AUTH_REQUIRED
VERIFICATION_REQUIRED
RATE_LIMITED
ACCESS_DENIED
SOURCE_UNAVAILABLE
PARSE_FAILED
NORMALIZE_FAILED
CANCELLED
UNKNOWN
```

平台自己的错误字符串只能作为诊断信息，不能成为上层业务分支条件。

### 限流

出现平台限流或拒绝访问时：

```text
立即停止本轮
→ 返回结构化错误
→ 上层调度器决定以后何时再次尝试
```

Sync Engine 本身不进行无限重试。

---

## 11. 数据写入边界

当前 Sync Engine 通过 `onBatch` 输出标准化订单：

```text
Sync Engine
→ UnifiedOrder[]
→ onBatch
```

未来由业务层决定：

```text
onBatch
├── SQLite Repository
├── PostgreSQL Repository
├── Sync Outbox
└── Order Event Generator
```

这样公共采集核心不会与某个数据库实现绑定。

---

## 12. PoC 接入方式

每个平台 PoC 通过后，只新增平台专用实现，不修改公共流程。

例如京东：

```text
PoC 证明：
登录可保持
订单页可访问
订单数据可稳定取得
        ↓
JdSource
        ↓
JDAdapter
        ↓
runShoppingSync()
        ↓
UnifiedOrder
```

平台接入至少需要验证：

1. 登录状态可以稳定判断
2. 用户验证可以人工完成
3. 订单列表可以增量读取
4. 一单多商品可以正确表达
5. 订单状态可以映射
6. 未完成订单可以再次刷新
7. 物流 / 履约信息可选补全
8. 平台改版时失败是可诊断的

---

## 13. 第一阶段正式接入顺序

当前建议：

```text
1. 京东
2. 淘宝 / 天猫
3. 拼多多
4. 美团
```

原因不是要求所有平台使用相同抓取方式，而是逐个平台证明 Adapter 能够独立接入公共架构。

---

## 14. 当前代码位置

```text
apps/desktop/src/services/shopping/
├── index.ts        # 公共导出
├── types.ts        # 领域模型与运行时校验
├── errors.ts       # 统一错误
├── source.ts       # 采集源契约
├── adapter.ts      # 平台 Adapter 契约
├── sync-state.ts   # 状态机类型
└── sync-engine.ts  # 公共同步编排
```

测试：

```text
apps/desktop/tests/shopping-core.test.ts
```

执行计划：

```text
docs/epic-29/execution-plan.md
```

---

## 15. 当前安全边界

公共核心明确不包含：

- Cookie 读取 / 导出
- Token 读取 / 导出
- 密码保存
- 验证码自动处理
- 浏览器远程控制
- 平台请求签名逆向
- 平台反自动化绕过

这些能力如果未来确有需要，必须在平台 PoC 与专门安全设计中单独评估，不能塞入通用 Sync Engine。
