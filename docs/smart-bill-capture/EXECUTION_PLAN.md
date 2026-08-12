# LifeTrace 智能截图记账执行方案

> 目标：参考 BeeCount 的“截图发现 → Vision 账单抽取 → 结构化交易 → 分类/账户匹配 → 入库”思路，在不复制 BeeCount 源码的前提下，将能力整合到 LifeTrace Android、Cloud、Windows/Tauri Desktop 与 Browser Client。
>
> 实施分支：`feature/smart-bill-capture`（LifeTrace 与 LifeTrace-finance 两仓库同名分支）。

## 1. 范围与原则

### 1.1 本次实现范围

1. **LifeTrace Cloud**
   - 新增统一的图片账单识别 API。
   - 默认支持智谱 `glm-4v-flash`，模型与 Base URL 可配置。
   - API Key 仅保存在 Cloud 环境变量，不下发客户端。
   - 图片只在请求生命周期内处理，默认不持久化原图。
   - 返回统一的结构化账单候选，不直接写用户交易表。

2. **LifeTrace Finance Android**
   - 支持从系统分享图片进入智能记账。
   - 支持用户显式开启“截图监听”；监听 MediaStore 新增截图，发现后调用 Cloud 识别。
   - Android 13+ 需要 `READ_MEDIA_IMAGES`；功能未授权时自动降级为“分享到 LifeTrace Finance”。
   - 识别结果先落为 `candidate/provisional`，复用现有待确认箱、分类、账户和 Outbox 同步。
   - 截图只用于当次识别；不把原图同步到 Cloud。

3. **Windows/Tauri Desktop**
   - 财务模块增加“图片记账”入口。
   - 用户选择本地截图后调用同一个 Cloud API。
   - 识别结果进入现有财务交易编辑/确认流程。

4. **Browser Client**
   - 财务模块增加图片上传识别入口。
   - 使用浏览器文件选择器选择截图；调用同一个 Cloud API。
   - 不引入浏览器端业务主数据缓存，仍以 Cloud 数据为准。

### 1.2 不做的事情

- 不复制 BeeCount Flutter/Dart 源码；仅复刻公开架构思想。
- 不使用 Root、LSPosed/Xposed 或 Accessibility Hook 微信/支付宝内部数据。
- 不长期保存微信/支付宝截图到 Cloud。
- 不把 GLM API Key 写入 APK、浏览器 JS 或 Desktop 前端。
- 第一版不做永久前台服务常驻；Android 自动截图监听只在应用进程存活且用户显式授权/开启时工作，可靠兜底路径是系统“分享图片到 LifeTrace Finance”。

## 2. 总体架构

```text
Android Screenshot / Share       Desktop File Picker       Browser File Picker
          |                              |                         |
          +------------------------------+-------------------------+
                                         |
                                         v
                              LifeTrace Cloud API
                           POST /api/v1/finance/capture/image
                                         |
                       MIME/size/auth/rate-limit validation
                                         |
                                         v
                           Vision Provider abstraction
                                         |
                               Zhipu GLM-4V-Flash
                                         |
                                         v
                           Strict JSON response parser
                                         |
                                         v
                            FinanceCaptureResult[]
                                         |
               +-------------------------+------------------------+
               |                         |                        |
             Android                  Desktop                  Browser
               |                         |                        |
        candidate + evidence       confirm/create tx       confirm/create tx
               |                         |                        |
               +-------------------------+------------------------+
                                         |
                                  finance.transaction
                                         |
                                      Sync/Cloud
```

## 3. Cloud 设计

### 3.1 配置

新增环境变量：

- `LIFETRACE_VISION_BASE_URL`，默认 `https://open.bigmodel.cn/api/paas/v4`
- `LIFETRACE_VISION_API_KEY`，必填后能力才启用
- `LIFETRACE_VISION_MODEL`，默认 `glm-4v-flash`
- `LIFETRACE_VISION_MAX_IMAGE_BYTES`，默认 10 MiB

### 3.2 API

`POST /api/v1/finance/capture/image`

请求：`multipart/form-data`

- `image`: PNG/JPEG/WebP
- 可选 `currentTime` / `timezone`

响应：

```json
{
  "provider": "zhipu",
  "model": "glm-4v-flash",
  "bills": [
    {
      "amountCents": 2850,
      "currency": "CNY",
      "type": "expense",
      "merchant": "瑞幸咖啡",
      "occurredAt": "2026-08-12T09:30:00+08:00",
      "accountHint": "招商银行储蓄卡",
      "categoryHint": "餐饮",
      "externalTransactionId": null,
      "confidence": 0.92
    }
  ]
}
```

非账单图片返回 `bills: []`，而不是 4xx。

### 3.3 Prompt/校验策略

- 先判断是否为真实账单/支付交易界面，非账单返回空数组。
- 支持一张截图识别多笔独立交易。
- `amountCents > 0` 为硬校验。
- `type` 仅允许 LifeTrace 已定义枚举。
- 时间解析失败时不伪造服务端时间：允许 `occurredAt = null`，由客户端按截图时间/当前时间补全并标为低置信度。
- Cloud 不相信模型返回的本地 `categoryId/accountId`；模型只能返回 hint，最终匹配由客户端或现有业务层完成。

### 3.4 安全

- 复用现有 Auth v1，要求 `finance:write`。
- 图片 MIME 与文件头双重限制。
- 限制请求体大小与超时。
- 不记录图片 Base64、OCR 文本、完整 Provider 响应到普通日志。
- 错误日志只记录 request id、provider/model、状态码、耗时。
- 对识别 API 增加独立限流。

## 4. Android 设计

### 4.1 图片入口

两条路径共用同一 `SmartCaptureService`：

1. **Share Receiver**：用户从微信/支付宝截图预览、相册等通过 Android 分享菜单发送给 LifeTrace Finance。
2. **Screenshot Monitor**：用户在设置中显式开启后，使用 `ContentObserver + MediaStore` 监听新增截图；通过文件名/路径关键词、创建时间和去重集合筛选新截图。

### 4.2 识别与落库

```text
image Uri/path
  -> SHA-256 + duplicate guard
  -> Cloud capture API
  -> CaptureBill
  -> 本地账户/分类 hint 匹配
  -> finance.transaction(status=candidate/provisional, source_type=vision_screenshot)
  -> finance.transaction_evidence(source_type=vision:glm-4v-flash)
  -> Outbox
  -> 待确认箱
```

### 4.3 数据库

尽量复用现有 `finance_transactions`、`finance_transaction_evidence`。
新增本地 `smart_capture_events` 表，仅保存：

- 图片哈希
- 来源（share / screenshot_monitor）
- provider/model
- capture 时间
- 识别结果状态
- transaction id 列表

不保存原图和 Provider 原始响应。

数据库升级到 v2，并提供显式 Migration，禁止因升级丢失现有财务数据。

### 4.4 UI/设置

- 设置页：智能截图记账开关、权限状态、Cloud Vision 可用状态。
- 待确认箱：显示“截图识别”来源、模型置信度。
- 分享图片后直接进入识别进度/结果，而不是当前的“仅缓存文件”占位实现。

## 5. Desktop / Browser 设计

### 5.1 共用前端服务层

新增 `financeCaptureApi.ts`：

- `captureFinanceImage(file)`
- 统一解析 Cloud response
- 统一错误类型：未登录、Cloud 未配置、模型未配置、图片过大、Provider 失败

### 5.2 Desktop

- 财务页增加“图片记账”。
- 使用 `<input type=file>`/Tauri 前端可访问文件对象的方式读取图片；不要求 Tauri Rust 新增模型能力。
- 调用 Cloud 后把候选结果送入已有交易创建逻辑。

### 5.3 Browser

- 使用同一 React 组件和 API service。
- 仅允许当前登录 Cloud 用户使用。
- 不把图片保存到 IndexedDB/localStorage。

## 6. 契约

新增跨端 DTO：

- `FinanceCaptureBill`
- `FinanceCaptureResponse`
- `FinanceCaptureError`

保持同步实体 `finance.transaction` / `finance.transaction_evidence` 不变；智能识别 API 是命令式辅助 API，不修改 Sync Protocol v1 的基本语义。

Android 仓库更新 contract snapshot，确保字段与 Cloud 契约一致。

## 7. 测试计划

### Cloud

- 未认证/缺 scope 拒绝。
- Vision 未配置返回明确错误。
- 非图片/超限图片拒绝。
- Provider 返回单笔、多笔、非账单、Markdown 包裹 JSON、非法金额时均有测试。
- Provider 5xx/超时不会泄漏 API Key 或图片内容。

### Android

- ShareReceiver 图片路径进入智能识别，而非仅文本占位。
- screenshot filename/path detection。
- 30 秒新图窗口与重复哈希去重。
- GLM JSON/Cloud response 解析。
- 本地 hint 匹配与 candidate 落库。
- Room v1 -> v2 Migration 保留既有交易。

### Desktop / Browser

- 图片选择和上传 service 单测。
- `bills=[]` 显示“未识别到账单”，不创建交易。
- 单笔/多笔结果正确进入确认流程。
- Desktop build + Browser build 均通过。

### 全量门禁

LifeTrace：

```bash
npm run test:desktop
npm run test:rust
npm run test:cloud
npm run contracts:check
npm run browser:build
```

LifeTrace-finance：

```bash
gradle :core:test :app:testDebugUnitTest :app:lintDebug :app:assembleDebug
```

## 8. 实施顺序

1. 写执行方案并建立同名功能分支。
2. Cloud：Provider/配置/DTO/Route/测试。
3. LifeTrace 前端：共用 capture API + Finance 图片入口 + Desktop/Browser 测试。
4. Android：网络 DTO/SmartCaptureService/ShareReceiver/截图监听/Room Migration/UI/测试。
5. 更新双方文档与 contract snapshot。
6. 创建两个 PR，等待 GitHub Actions 全部通过；不自动合并 `main`。

## 9. 验收标准

- Android 分享一张微信/支付宝支付截图可得到候选账单并进入待确认箱。
- Android 开启截图监听并授权后，新截图可自动触发同一识别流程；关闭后不监听。
- Desktop 和 Browser 都能选择账单截图并生成候选交易。
- 四端使用同一 Cloud Vision API 和同一结构化响应契约。
- GLM API Key 只存在 Cloud 环境变量。
- 原图默认不持久化到 Cloud，也不进入 Sync 数据。
- 非账单截图不会创建交易。
- 多笔账单截图可以产生多条候选。
- 所有既有测试和新增测试通过。
- 所有改动停留在 `feature/smart-bill-capture` / PR，不直接合并 `main`。
