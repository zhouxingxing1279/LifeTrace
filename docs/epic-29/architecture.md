# EPIC-29 Android 订单采集架构

> 日期：2026-08-10  
> 状态：Android-first 方案确定，平台 PoC 尚未完成

## 1. 架构结论

订单采集采用独立 Android Order App 作为第一采集端。云端不运行购物平台常驻浏览器作为默认方案，不保存购物平台认证凭据。

```text
                        Android Order App
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
     WebView Auth       Notification Trigger   App Lifecycle
          │                    │                    │
          └──────────────┬─────┴──────────────┬────┘
                         ▼                    ▼
                    Platform Manager      Sync Scheduler
                         │
        ┌────────────────┼─────────────────┐
        ▼                ▼                 ▼
 TaobaoAdapter        JDAdapter      MeituanAdapter ...
        │                │                 │
        └────────────────┼─────────────────┘
                         ▼
                     Fetch Engine
             ┌───────────┼───────────┐
             ▼           ▼           ▼
          Native       WebView       DOM
          HTTP          Fetch       Fallback
             └───────────┼───────────┘
                         ▼
                    Raw Platform Data
                         ▼
                      Normalizer
                         ▼
                     UnifiedOrder
                         ▼
                        Room
                         │
              ┌──────────┴──────────┐
              ▼                     ▼
         Android UI           LifeTrace Sync
```

## 2. 生命周期

主动抓取只在应用前台执行。

```text
BACKGROUND / CLOSED
        │
        │ app foreground
        ▼
INITIAL_REFRESH
        │
        ├── 普通订单 → NORMAL_TRACKING（5 min）
        │
        └── 活跃外卖 → REALTIME_DELIVERY（10～20 s）
                         │
                         └── 完成 / 取消 → NORMAL_TRACKING

任意前台状态
        │
        │ app background
        ▼
STOPPED
```

重新回到前台时，不等待上一轮定时器，而是立即执行一次刷新，再重新建立周期。

## 3. 登录与认证上下文

### WebView 的职责

WebView 只负责：

- 打开平台官网登录页面；
- 由用户正常完成密码、短信、扫码、安全确认等流程；
- 持久化 Cookie 及站点本地存储；
- 在必要时提供平台自己的页面 JS 环境。

### 凭据边界

以下内容只能留在 Android 本地：

- Cookie；
- Access / Session Token；
- WebView Profile；
- LocalStorage / IndexedDB 中的认证数据；
- 可能用于平台请求的临时签名上下文。

不得上传到普通 LifeTrace 业务数据库、云端日志、诊断包或其他设备。

## 4. Fetch Engine

Fetcher 按平台 PoC 结果选择，不强迫所有平台使用同一路径。

### 4.1 Native HTTP/API

首选方案。WebView 完成登录后，由原生网络层复用必要认证状态，请求订单列表、详情、物流或外卖状态 JSON。

优点：

- 资源消耗低；
- 不需要完整渲染页面；
- 适合 5 分钟普通刷新和 10～20 秒外卖状态刷新。

### 4.2 WebView Context Fetch

如果平台请求依赖页面 Token、JS 运行环境或浏览器上下文，则在可信 origin 的 WebView 内执行 fetch/XHR，再通过受 origin 限制的消息桥返回结果。

### 4.3 DOM Fallback

只有前两种方案无法稳定取得结构化数据时才解析 DOM。DOM 方案必须封装在平台 Adapter 内，并把页面结构变化视为可诊断的 `PARSE_FAILED`，不能污染业务层。

## 5. 平台 Adapter

公共接口建议：

```text
PlatformAdapter
├── checkAuth()
├── fetchOrderPage(cursor, range)
├── fetchOrderDetail(orderId)?
├── refreshOrders(orderIds)?
├── fetchActiveDeliveries()?
├── normalize(raw)
└── classifyError(error)
```

Adapter 负责平台 URL、分页参数、原始字段、订单状态、物流状态、退款状态等平台差异。

上层不允许直接依赖淘宝、京东、美团原始字段。

## 6. 统一订单模型

稳定业务键：

```text
platform + accountId + platformOrderId
```

核心模型：

```text
UnifiedOrder
├── platform
├── accountId
├── platformOrderId
├── orderedAt
├── merchantName
├── status
├── amountMinor
├── OrderItem[]
├── Fulfillment[]
├── Refund[]
├── sourceUpdatedAt?
└── lastSeenAt
```

金额统一使用最小货币单位整数。

履约统一使用 `Fulfillment`，兼容普通快递、京东配送、外卖、本地配送、到店自取和虚拟商品。

## 7. 首次历史回填

第一次使用平台连接时，用户可以选择：

```text
最近 1 个月
最近 1 年
```

同步过程：

```text
start backfill
→ fetch page
→ normalize
→ Room upsert
→ 保存 checkpoint / cursor
→ fetch next page
→ 到达时间边界
→ mark initial_sync_completed
```

每一页成功后立即事务性写入 Room 并更新 checkpoint，确保 App 被杀掉、网络中断或用户退出后可以继续，而不是重头抓取。

## 8. 后续增量同步

后续打开 App 不重新扫描全部历史，使用三条增量路径。

### 8.1 新订单扫描

从最新订单页向后扫描，遇到稳定的已知区域后停止。停止条件不能只依赖“遇到第一条旧订单”，应至少允许 Adapter 使用连续已知页 / 时间边界等安全规则。

### 8.2 重叠校验窗口

最近一段时间的订单允许重复拉取并 UPSERT，例如最近 7～30 天。该窗口用于捕获：

- 退款；
- 售后；
- 延迟发货；
- 平台异步修正。

具体窗口由平台 Adapter 配置。

### 8.3 活跃订单刷新

所有尚未终态的订单独立进入刷新集合，例如：

```text
待付款
待发货
运输中
配送中
退款 / 售后中
```

即使这些订单早于最新列表的停止边界，也必须可单独刷新。

## 9. Room 与同步状态

建议至少维护：

```text
orders
├── platform
├── account_id
├── platform_order_id
├── ordered_at
├── status
├── is_active
├── source_updated_at
├── last_seen_at
└── raw_hash?

shopping_sync_state
├── platform
├── account_id
├── initial_sync_completed
├── initial_range_start
├── initial_cursor
├── last_success_at
└── source_cursor
```

`raw_hash` 可用于判断关键字段没有变化时跳过无意义的数据库更新和 UI 事件。

## 10. 通知触发器

`NotificationListenerService` 只用于识别“某个平台可能发生订单变化”。

触发规则：

```text
package 属于已支持平台
+ 通知满足订单 / 配送相关规则
→ enqueue immediate refresh signal
```

当 App 前台时，立即调用对应 Adapter 刷新。

当 App 后台时，不启动长期轮询；只记录轻量待检查标记，下一次进入前台立即验证。

通知正文不能作为最终业务状态，因为通知可能缺订单号、金额或完整状态，也可能因为文案变化而失真。

## 11. 外卖实时追踪

入口有两个：

1. App 进入前台后的首次同步发现活跃外卖；
2. App 前台时收到外卖相关通知并确认存在活跃订单。

```text
VERIFY_ACTIVE_DELIVERY
        ↓
有活跃订单？
   ├── 否 → NORMAL_TRACKING
   └── 是 → REALTIME_DELIVERY
              ↓
           10～20 秒状态刷新
              ↓
       通知到达时立即抢先刷新
              ↓
        completed / cancelled
              ↓
         NORMAL_TRACKING
```

如果 PoC 发现平台自身存在稳定 WebSocket / SSE / 长轮询，可替换周期轮询，但上层仍保持同一 `RealtimeDeliveryTracker` 契约。

## 12. 云端同步边界

Android 完成采集和标准化后，可以把 `UnifiedOrder` 作为普通 LifeTrace 业务数据同步到云端。

云端可以保存：

- 订单；
- 商品；
- 履约 / 物流事件；
- 退款；
- 与财务流水的关联。

云端不得保存：

- Cookie；
- Token；
- WebView Profile；
- 平台登录密码；
- 用于复现平台认证环境的私有浏览器状态。

## 13. 错误模型

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

登录失效、人机验证和限流都必须停止当前高频流程，禁止无限自动重试。

## 14. PoC 顺序

第一阶段优先验证：

```text
1. 美团：登录、当前订单、外卖状态实时刷新、通知触发
2. 京东：历史回填、增量订单、物流状态
3. 淘宝 / 天猫：历史回填、增量订单、物流状态
4. 拼多多：同一 Adapter 契约验证
```

每个平台 PoC 必须回答：

- 登录态是否可稳定保存在 WebView；
- 哪种 Fetcher 最稳定；
- 历史订单分页如何停止；
- 活跃订单如何刷新；
- 哪些异常要求用户重新验证；
- 页面 / 接口变化是否可被诊断。
