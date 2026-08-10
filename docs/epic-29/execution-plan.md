# EPIC-29 购物订单公共架构执行计划

> 状态：执行中  
> 日期：2026-08-10  
> 分支：`agent/epic29-shopping-core`  
> 目标：在任何淘宝 / 京东 / 拼多多 / 美团 PoC 抓取逻辑接入之前，先建立稳定、可测试、与平台实现解耦的购物订单公共架构。

## 1. 本次范围

本次只实现“平台无关”的公共骨架，不实现任何真实平台抓取。

### 本次实现

- 统一购物平台标识与连接状态
- 统一订单、商品、履约 / 物流、退款数据模型
- 平台 Adapter 契约
- Collector Source 契约
- 登录 / 验证状态模型
- 增量同步游标模型
- 同步状态机
- 公共同步编排器
- 去重与标准化结果约束
- 单个平台失败隔离
- 可测试的内存测试实现
- 单元测试
- EPIC-29 架构文档

### 本次不实现

- 淘宝真实登录
- 京东真实登录
- 拼多多真实登录
- 美团真实登录
- WebView2 / Chromium / Playwright 自动化
- DOM Selector
- XHR / Fetch 接口解析
- Cookie、Session、Token 持久化
- 滑块、短信、扫码等验证处理 UI
- 云端浏览器 Worker
- 数据库 migration
- 财务流水自动匹配
- 定时任务调度
- Order Center 正式 UI

这些能力必须在 PoC 证明对应平台采集路线可行后，通过 Adapter / Source 接口接入公共框架。

---

## 2. 设计原则

### 2.1 平台差异只存在于 Adapter / Source

公共层不得包含：

```text
淘宝 URL
京东 URL
拼多多 DOM Selector
美团订单字段名
平台 Cookie 名称
平台验证码规则
```

平台专用逻辑后续放入：

```text
TaobaoAdapter
JDAdapter
PddAdapter
MeituanAdapter
```

公共编排器只依赖接口。

### 2.2 抓取方式与平台 Adapter 解耦

同一平台未来可能存在：

```text
Browser XHR
Browser DOM
Official API
Cloud Browser Worker
Windows WebView Collector
```

因此 Adapter 不直接绑定浏览器实现，底层通过 Source 契约提供原始数据。

### 2.3 统一领域模型优先

所有平台最终必须转换成统一模型：

```text
UnifiedOrder
├── OrderItem[]
├── Fulfillment[]
│   └── TrackingEvent[]
└── Refund[]
```

上层 Order Center、Cloud Sync、财务匹配不得依赖平台原始字段。

### 2.4 验证是同步状态，不是异常重试

出现登录失效、滑块、短信或扫码验证时：

```text
SYNCING
→ VERIFICATION_REQUIRED
→ WAITING_FOR_USER
```

禁止进入自动高频重试。

### 2.5 增量同步优先

公共同步模型必须支持：

- 首次历史同步
- 后续增量同步
- 已知订单边界停止
- 未完成订单状态刷新
- 中断后继续
- 重复执行幂等

---

## 3. 目标目录

第一阶段公共实现放在桌面端共享 TypeScript 服务层，保持纯逻辑、无 DOM、无 Tauri API、无浏览器依赖：

```text
apps/desktop/src/services/shopping/
├── index.ts
├── types.ts
├── adapter.ts
├── source.ts
├── sync-state.ts
├── sync-engine.ts
└── errors.ts
```

测试：

```text
apps/desktop/tests/
└── shopping-core.test.ts
```

文档：

```text
docs/epic-29/
├── execution-plan.md
└── architecture.md
```

后续 PoC 通过后再增加：

```text
apps/desktop/src/services/shopping/adapters/
├── taobao.ts
├── jd.ts
├── pdd.ts
└── meituan.ts
```

如果最终选择云端 Chromium / Playwright Collector，则平台采集实现可以位于独立 Worker；只需保证输出符合本公共契约。

---

## 4. 公共领域模型

### ShoppingPlatform

第一阶段：

```text
taobao
jd
pdd
meituan
```

模型允许后续扩展，不在业务逻辑中写死数组分支。

### UnifiedOrder

最小字段：

- `platform`
- `platformOrderId`
- `orderedAt`
- `merchantName`
- `status`
- `currency`
- `originalAmountMinor`
- `discountAmountMinor`
- `shippingFeeMinor`
- `paidAmountMinor`
- `items`
- `fulfillments`
- `refunds`
- `updatedAt`

金额统一使用最小货币单位整数。

### Fulfillment

统一覆盖：

```text
parcel
platform_delivery
local_delivery
pickup
virtual
none
```

字段包括：

- 承运商
- 运单号
- 平台履约 ID
- 当前状态
- 预计送达时间
- 发货 / 签收时间
- 最新事件
- 完整 TrackingEvent 列表

### VerificationRequirement

统一表示：

```text
login
slider
sms
qr
security_confirmation
unknown
```

公共层只识别“需要用户处理”，不尝试自动绕过。

---

## 5. Adapter 与 Source 契约

### ShoppingSource

职责：

- 保持或访问某个平台认证上下文
- 返回平台原始订单页 / 接口结果
- 提供当前认证 / 验证状态
- 不负责转换成 LifeTrace 领域对象

### ShoppingAdapter

职责：

- 判断平台登录状态
- 从 Source 获取一页订单
- 可选获取订单详情
- 可选获取履约 / 物流详情
- 将原始数据标准化为 `UnifiedOrder`
- 根据平台语义判断下一页 / 增量停止条件

Adapter 不负责：

- 数据库写入
- Cloud Sync
- UI
- 全局调度
- 财务匹配

---

## 6. Sync Engine

公共同步引擎负责：

```text
start
↓
check connection
↓
check verification
↓
load cursor
↓
fetch page
↓
normalize
↓
dedupe
↓
emit batch
↓
advance cursor
↓
continue / stop
```

状态：

```text
idle
checking_connection
syncing
verification_required
paused
completed
failed
```

### 同步停止条件

- Adapter 明确返回结束
- 达到已知订单边界
- 当前页没有新增或变化数据且 Adapter 判断可以停止
- 需要用户验证
- Session 失效
- 平台拒绝访问
- 调用方主动取消

### 重试原则

公共引擎不做无限重试。

第一版：

- 单次同步由调用方显式启动
- 普通瞬时错误最多由上层调度器决定是否重试
- `verification_required` 不重试
- `auth_expired` 不重试
- `rate_limited` 不立即重试

---

## 7. 错误模型

统一错误代码：

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

平台原始错误可以作为脱敏 metadata 保留，但不能让上层业务依赖平台字符串。

---

## 8. 测试计划

使用纯 TypeScript 单元测试，不访问真实购物平台。

### 必须覆盖

1. Adapter 输出能够进入统一模型
2. 同一 `platform + platformOrderId` 可以生成稳定去重键
3. 不同平台相同订单号不会冲突
4. 多页增量同步按顺序推进 cursor
5. 遇到已知订单边界停止
6. 遇到 `VERIFICATION_REQUIRED` 立即暂停，不继续请求
7. 登录失效返回 `AUTH_REQUIRED`
8. 单个平台异常以结构化失败返回
9. 已完成同步返回最终 cursor
10. 取消同步后不继续获取下一页
11. 金额模型只接受整数最小货币单位
12. 一个订单支持多个商品、多个履约记录和多个退款记录

### 回归门禁

分支合并 `main` 前至少要求：

```text
npm --prefix apps/desktop run lint
npm --prefix apps/desktop run test:unit
npm --prefix apps/desktop run web:build
npm --prefix apps/desktop run browser:build
```

如果 GitHub Actions 对该分支没有自动触发，则通过 PR 触发已有 `Browser Web` workflow；CI 全部通过后才允许合并。

---

## 9. 文档计划

新增 `docs/epic-29/architecture.md`，记录：

- 公共层边界
- Adapter / Source 分层
- UnifiedOrder / Fulfillment 模型
- Sync Engine 状态机
- Verification Relay 未来接入点
- Windows Collector 与 Cloud Collector 的兼容方式
- PoC 后各平台接入步骤

同时不把实现细节继续塞入 `docs/roadmap.md`；路线图保持 EPIC 粒度。

---

## 10. PoC 后接入顺序

PoC 不属于本次公共架构提交。

PoC 通过后按以下方式接入：

### 第一优先：京东

- 登录持久化
- 订单列表 Source
- JD Adapter
- 少量订单增量同步
- 物流 / 履约详情

### 第二优先：淘宝 / 天猫

### 第三优先：拼多多

### 第四优先：美团

每个平台都必须先完成自己的 PoC，再进入正式 Adapter。

---

## 11. 合并策略

1. 从最新 `main` 创建 `agent/epic29-shopping-core`
2. 先提交本执行计划
3. 再实现公共代码
4. 增加单元测试
5. 增加架构文档
6. 创建 PR 到 `main` 触发 CI
7. 检查全部测试 / 构建结果
8. 只有 CI 通过后合并到 `main`
9. CI 失败则修复后重新验证，禁止带红合并

---

## 12. 本次完成定义

本次公共架构完成必须同时满足：

- [ ] 不包含任何平台真实抓取逻辑
- [ ] 公共类型可表达淘宝 / 京东 / 拼多多 / 美团的订单差异
- [ ] Adapter 和 Source 契约稳定
- [ ] Sync Engine 支持增量、多页、验证暂停和取消
- [ ] 单元测试覆盖核心状态机
- [ ] `architecture.md` 完成
- [ ] TypeScript 类型检查通过
- [ ] 单元测试通过
- [ ] Web build 通过
- [ ] Browser build 通过
- [ ] GitHub CI 通过
- [ ] 最终合并到 `main`
