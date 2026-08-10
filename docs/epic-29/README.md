# EPIC-29：购物订单聚合与 Android 本地采集

> 状态：规划已更新，等待平台 PoC 与 Android 实现  
> 日期：2026-08-10

## 目标

将淘宝 / 天猫、京东、拼多多、美团等个人消费平台的订单统一采集到 LifeTrace，同时把购物平台登录态和采集执行尽量留在 Android 本地。

本 Epic 的第一阶段不再以云端 Chromium 作为主采集器，而采用独立 Android Order App：

```text
Android Order App
├── WebView：用户登录 / 验证
├── Platform Adapter：平台差异
├── Fetch Engine：订单数据采集
├── Room：本地订单数据库
├── Sync Engine：全量 + 增量
├── Notification Trigger：订单变化触发
└── Realtime Delivery Tracker：外卖前台实时追踪
```

采集后的订单转换成 `UnifiedOrder`，再按需要同步到 LifeTrace Cloud / Windows；Cookie、Token、WebView Profile 等购物平台认证凭据不得上传云端。

## 核心产品决策

### 1. 前台按需采集

订单 App 的主动采集与用户关注状态保持一致：

```text
App 未打开 / 进入后台
→ 不主动抓取订单

App 进入前台
→ 立即同步全部平台
→ 普通平台每 5 分钟刷新

发现活跃外卖
→ 10～20 秒级状态刷新
→ 订单完成后恢复普通刷新

App 再次进入后台
→ 停止所有主动轮询
```

### 2. 首次回填 + 后续增量

首次使用时允许用户选择最近 1 个月或最近 1 年作为历史回填范围。历史订单分页写入 Room，并保存断点游标。

首次回填完成后，不再反复扫描全部历史：

- 扫描最新订单页，发现新增订单；
- 对最近时间窗口进行重叠校验，避免退款 / 售后等延迟变化漏失；
- 单独刷新所有仍处于活跃状态的订单；
- 已完成且足够久远的订单不主动重复抓取。

### 3. Android 本地登录态

登录和安全验证通过 Android WebView 由用户本人完成。平台认证信息只保存在 Android 设备：

```text
Cookie
Token
LocalStorage / IndexedDB
WebView Profile
```

正常采集优先复用已有认证上下文，不自动绕过验证码、滑块、短信或安全确认。

### 4. 三层数据获取策略

每个平台 Adapter 按可行性选择：

```text
Native HTTP/API
      ↓ 不可稳定复现
WebView 页面上下文 fetch/XHR
      ↓ 仍不可用
DOM 解析兜底
```

不读取其他 App 的私有数据库，不以 Root 为前提，也不把 Accessibility 自动操作作为核心方案。

### 5. 外卖通知驱动实时模式

`NotificationListenerService` 只作为“可能有订单变化”的触发器，不把通知文本作为订单真值。

当 App 处于前台时：

```text
收到美团等外卖通知
→ 立即刷新对应平台
→ 确认存在进行中外卖
→ 启动 RealtimeDeliveryTracker
→ 10～20 秒刷新当前订单状态
→ 收到新的相关通知时立即抢先刷新
→ 已送达 / 已取消后停止实时模式
```

App 处于后台时，通知可以记录为待检查信号，但不启动持续高频抓取。

## 数据边界

LifeTrace Cloud 可以保存标准化后的订单业务数据，但不得保存购物平台登录凭据。

```text
Android 本地：
- Cookie / Token / WebView Profile
- 平台原始认证上下文
- 原始抓取缓存（如需要，受生命周期控制）

LifeTrace 业务层：
- UnifiedOrder
- OrderItem
- Fulfillment / TrackingEvent
- Refund
- SyncState
```

## 依赖

- EPIC-02 统一领域模型与同步协议
- EPIC-05 客户端同步核心
- EPIC-06 手工记账、自动记账与账单对账
- EPIC-17 安全、隐私与数据生命周期
- EPIC-19 监控、日志、客户端诊断与运维

## 验收标准

- [ ] Android Order App 能由用户正常登录至少一个购物平台并保持本地登录态
- [ ] 首次可以回填最近 1 个月或最近 1 年订单，并支持中断续传
- [ ] 后续打开 App 时只做增量扫描、重叠校验和活跃订单刷新
- [ ] 普通订单在 App 前台时可按 5 分钟级同步
- [ ] 活跃外卖在 App 前台时可进入 10～20 秒级追踪
- [ ] 收到相关外卖通知时可以立即触发一次真实订单状态刷新
- [ ] App 进入后台后停止主动订单轮询
- [ ] 同一 `platform + accountId + platformOrderId` 重复抓取不会产生重复订单
- [ ] Cookie、Token 和 WebView Profile 不上传 LifeTrace Cloud
- [ ] 平台要求人机验证时暂停并交给用户处理
- [ ] 平台采集失败不会破坏已经保存的历史订单

详细设计见：

- `architecture.md`
- `execution-plan.md`
