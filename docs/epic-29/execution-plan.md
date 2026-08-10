# EPIC-29 Android 订单采集执行计划

> 状态：待执行  
> 日期：2026-08-10  
> 目标：在现有 UnifiedOrder / Shopping Adapter 公共核心基础上，把订单采集正式收敛到独立 Android Order App，并实现首次历史回填、后续增量同步和前台外卖实时追踪。

## 1. 实施范围

### 本 Epic 实现

- 独立 Android Order App 基础工程；
- WebView 登录与认证状态管理；
- 平台 Adapter / Fetcher 契约；
- Room 订单与同步状态库；
- 首次 1 个月 / 1 年历史回填；
- 中断续传 checkpoint；
- 后续增量同步；
- 最近窗口重叠校验；
- 活跃订单刷新；
- App 前台生命周期调度；
- NotificationListener 触发刷新；
- 美团活跃外卖实时追踪；
- 标准化订单同步到 LifeTrace；
- 登录凭据本地隔离；
- 单元、集成与 Android 端测试。

### 第一阶段不实现

- 云端常驻 Chromium 主采集；
- 后台 24 小时订单轮询；
- WorkManager 强行实现 5 分钟周期；
- Accessibility 自动操作第三方 App；
- Root 读取第三方 App 私有数据库；
- 自动破解滑块 / 验证码；
- 购物平台凭据上传云端；
- 所有平台同时并发高频抓取。

## 2. 阶段 A：Android Order App 骨架

建议目录：

```text
apps/order-android/
├── app/
├── data/
├── domain/
├── platform/
├── sync/
└── ui/
```

任务：

- [ ] Kotlin + Jetpack Compose 基础工程
- [ ] Coroutines / Flow
- [ ] Room
- [ ] 生命周期感知的 `OrderSyncManager`
- [ ] `PlatformAdapter` 接口
- [ ] `FetchEngine` 接口
- [ ] `UnifiedOrder` Android 映射
- [ ] 结构化错误模型

## 3. 阶段 B：认证与 WebView

- [ ] 平台连接页面
- [ ] 每个平台独立登录入口
- [ ] WebView Cookie / 本地存储持久化
- [ ] 登录失效检测
- [ ] 用户主动清除平台登录状态
- [ ] Debug 构建允许 WebView DevTools 调试
- [ ] Release 构建关闭 WebView 调试
- [ ] 原生消息桥限制可信 origin
- [ ] Cookie / Token 不进入普通日志

验收：用户可以正常完成平台登录，关闭并重新打开 Order App 后仍能恢复允许持久化的登录状态。

## 4. 阶段 C：平台 PoC

每个平台先完成最小 PoC，再进入正式 Adapter。

### 美团 PoC

- [ ] 登录保持
- [ ] 当前订单列表
- [ ] 当前外卖订单详情
- [ ] 订单状态 / 预计时间数据来源
- [ ] 10～20 秒刷新可行性
- [ ] NotificationListener 识别相关通知
- [ ] 通知触发后立即刷新
- [ ] 已送达 / 已取消终态识别

### 京东 PoC

- [ ] 历史订单分页
- [ ] 订单详情
- [ ] 活跃物流状态
- [ ] 增量停止边界

### 淘宝 / 天猫 PoC

- [ ] 历史订单分页
- [ ] 订单详情
- [ ] 物流 / 退款状态
- [ ] 增量停止边界

### 拼多多 PoC

- [ ] 在公共 Adapter / Fetcher 契约下验证可行性

每个平台需要明确记录最终 Fetch Mode：

```text
native-http
webview-fetch
dom-fallback
```

## 5. 阶段 D：Room 数据模型

- [ ] `orders`
- [ ] `order_items`
- [ ] `fulfillments`
- [ ] `tracking_events`
- [ ] `refunds`
- [ ] `shopping_sync_state`
- [ ] `platform_accounts`
- [ ] 唯一约束 `platform + account_id + platform_order_id`
- [ ] UPSERT
- [ ] 订单终态 / 活跃态标记
- [ ] migration 测试

## 6. 阶段 E：首次历史回填

- [ ] 用户选择最近 1 个月或最近 1 年
- [ ] 分页抓取并逐页写 Room
- [ ] 每页成功后保存 checkpoint
- [ ] 到达时间边界后标记完成
- [ ] App 被杀掉后可以继续
- [ ] 网络中断后可以继续
- [ ] 重复执行不会产生重复订单
- [ ] 显示回填进度和取消入口

验收：在中途退出 App 后重新打开，历史同步从已确认 checkpoint 继续，而不是重头抓取。

## 7. 阶段 F：增量同步

- [ ] App 进入前台立即刷新全部已连接平台
- [ ] 最新订单列表增量扫描
- [ ] 连续已知区域安全停止
- [ ] 最近 7～30 天可配置重叠校验
- [ ] 活跃订单独立刷新
- [ ] `last_success_at` / `source_cursor` 更新
- [ ] 普通平台前台每 5 分钟重新执行增量
- [ ] App 进入后台停止周期任务
- [ ] 回到前台立即刷新，不等待旧定时器

## 8. 阶段 G：通知驱动与外卖实时模式

- [ ] 用户显式授予通知访问权限
- [ ] 按 package 识别支持平台
- [ ] 区分订单 / 配送通知与营销通知
- [ ] 通知只生成 refresh signal，不直接改订单真值
- [ ] App 前台收到相关通知后立即刷新
- [ ] App 后台不启动持续高频抓取
- [ ] 发现活跃外卖后启动 `RealtimeDeliveryTracker`
- [ ] 默认 10～20 秒刷新当前订单
- [ ] 收到新通知时抢先刷新并重置周期
- [ ] 终态后自动停止实时模式
- [ ] 连续异常 / 超时后退出高频模式并展示错误

## 9. 阶段 H：LifeTrace 集成

- [ ] Android 本地 `UnifiedOrder` 写入 Sync Outbox
- [ ] 同步订单、商品、履约和退款业务数据
- [ ] Cloud 端按用户隔离
- [ ] 多设备重复同步同一订单仍保持幂等
- [ ] 与财务流水匹配服务对接
- [ ] 不同步平台 Cookie / Token / Profile

## 10. 测试与门禁

### 单元测试

- [ ] 平台 Adapter 标准化
- [ ] 去重键
- [ ] backfill 时间边界
- [ ] checkpoint 恢复
- [ ] 增量停止规则
- [ ] 重叠窗口
- [ ] 活跃订单刷新
- [ ] App 前后台状态切换
- [ ] 通知触发过滤
- [ ] 外卖实时状态机
- [ ] 终态自动停止

### 集成测试

- [ ] WebView 登录状态恢复
- [ ] Room migration
- [ ] 中断续传
- [ ] 网络错误恢复
- [ ] 登录过期
- [ ] 人机验证暂停
- [ ] 平台限流暂停
- [ ] Cloud Sync 凭据泄漏扫描

### 发布门禁

- [ ] Android lint
- [ ] Android unit test
- [ ] Android build
- [ ] LifeTrace Sync 兼容测试
- [ ] 日志敏感字段扫描
- [ ] 文档与实现一致性检查

## 11. 完成定义

EPIC-29 完成必须同时满足：

- [ ] 至少美团、京东、淘宝 / 天猫完成真实 Android PoC
- [ ] 至少一个平台完成 1 年历史回填测试
- [ ] 再次打开 App 时可以稳定执行增量同步
- [ ] App 后台时没有主动 5 分钟轮询
- [ ] 活跃外卖在前台可 10～20 秒级更新
- [ ] 通知触发可以缩短外卖状态变化等待时间
- [ ] 购物平台凭据只留在 Android 本地
- [ ] 标准化订单可以同步给 LifeTrace 其他端
- [ ] 核心测试和构建门禁通过
