# LifeTrace 智能截图记账执行方案

> 最终边界：尽量保持 BeeCount 当前“截图发现 → AutoBillingService → AiBookkeeper → Vision Provider → JSON Parser → BillCreationService → 入库”的架构，但把整条识别链放在 `LifeTrace-finance` Android 客户端。LifeTrace Cloud、Windows/Tauri Desktop、Browser 不处理原始截图，只消费同步后的结构化财务数据。
>
> 实施分支：两个仓库均为 `feature/smart-bill-capture`。

## 1. 仓库职责

### `LifeTrace-finance`

负责新增账单与图片识别：

- 系统图片分享；
- MediaStore 新截图监听；
- Android 直接调用 Vision API；
- 默认与 BeeCount 一致使用智谱 GLM / `glm-4v-flash`；
- API Key 使用 Android Keystore 加密，仅保存在手机；
- bill guard、多笔交易解析、JSON 容错；
- 本地账户/分类映射；
- 与支付通知 candidate 对账去重；
- 写入现有 `finance.transaction`；
- 复用 Outbox / Sync Engine 同步。

### `LifeTrace`

保持现有职责：

- Cloud：同步与鉴权，不新增 Vision 图片 API；
- Desktop：查看、编辑、统计财务数据；
- Browser：查看、编辑、统计财务数据；
- 不存储 Vision API Key；
- 不接收支付截图；
- 不增加 OCR/VLM 依赖。

## 2. 总体数据流

```text
微信 / 支付宝 / 银行支付截图
              |
     +--------+--------+
     |                 |
 Android 分享      MediaStore 监听
     |                 |
     +--------+--------+
              |
       AutoBillingService
              |
          AiBookkeeper
              |
   DefaultAiExtractionEngine
              |
         PromptBuilder
              |
      AiProviderFactory
              |
 Android -> Vision API 直接调用
              |
      JsonResponseParser
              |
           BillInfo[]
              |
      BillCreationService
              |
  candidate / provisional
              |
       LifeTrace Outbox
              |
         Cloud Sync
              |
     +--------+--------+
     |                 |
 Desktop 查看       Browser 查看
```

原始图片只存在于 Android MediaStore 或 app 临时缓存。LifeTrace Sync 不包含图片。

如果用户在尚未配置 Vision API Key 时先分享截图，Android 会把临时缓存路径保存在 App 私有 `PendingShareStore`；该路径不会通过 exported Activity 的 Intent extra 暴露。保存 Vision 配置后一次性 consume 并继续识别，新的 pending 图片会清理旧临时文件。

## 3. BeeCount 架构映射

LifeTrace Android 使用原生 Kotlin 重写以下职责，不复制 BeeCount Dart/Kotlin 源码：

```text
ScreenshotObserver            -> ScreenshotObserver
ScreenshotMonitorService       -> ScreenshotMonitorService
ImageShareHandlerService       -> ShareReceiverActivity
AutoBillingConfig              -> AutoBillingConfig
AutoBillingService             -> AutoBillingService
AiBookkeeper                   -> AiBookkeeper
DefaultAiExtractionEngine      -> DefaultAiExtractionEngine
AIProviderFactory.vision       -> AiProviderFactory.vision
AIServiceProviderConfig        -> AiServiceProviderConfig / AiSettingsStore
PromptBuilder.billGuardForImage-> PromptBuilder.billGuardForImage
JsonResponseParser             -> JsonResponseParser
BillCreationService            -> BillCreationService
processed screenshot memory    -> ProcessedImageStore
```

`PendingShareStore` 是 LifeTrace 为 Android exported 设置入口额外增加的私有安全交接层，不改变 BeeCount 主识别管线。

## 4. Android Vision Provider

默认内置配置：

```text
providerId  = zhipu_glm
baseUrl     = https://open.bigmodel.cn/api/paas/v4
visionModel = glm-4v-flash
```

配置原则：

- Base URL 与 Vision Model 可在 Android 修改；
- API Key 通过 Android Keystore AES/GCM 加密；
- Key 不写 BuildConfig、源码、Room、Outbox 或 Cloud；
- Provider 请求使用现有 OkHttp；
- 图片以 Base64 放入 Vision `image_url` content；
- 最大图片 10 MiB；支持 PNG/JPEG/WebP；
- 实际 HTTP 调用由 `AutoBillingService` 的 `Dispatchers.IO` 协程域发起，不阻塞主线程。

## 5. Screenshot Monitor

与 BeeCount 行为对齐：

- `ContentObserver` 监听 `MediaStore.Images.Media.EXTERNAL_CONTENT_URI`；
- 仅处理最近约 30 秒新增媒体；
- observer debounce 约 500 ms；
- 文件名/路径关键词：`screenshot`、`截屏`、`截图`、`screen_shot`、`screen shot`；
- 自动监听必须由用户显式开启；
- Android 13+ 使用 `READ_MEDIA_IMAGES`；
- Android 12 及以下使用 `READ_EXTERNAL_STORAGE`；
- 第一版为进程生命周期监听，不使用永久 foreground service；
- 分享图片入口不依赖媒体读取权限，是稳定 fallback；
- MediaStore 按 `DATE_ADDED DESC` 排序并读取第一条，不在 sortOrder 拼接非标准 `LIMIT`。

## 6. 自动记账

`AutoBillingService`：

1. 检查 Provider/Key；
2. 等待图片写入完成；
3. 文件头校验；
4. SHA-256 去重；
5. `AiBookkeeper.fromImage`；
6. 非账单 `[]` 直接结束；
7. 有账单时交给 `BillCreationService`；
8. 成功后写入现有 Outbox；
9. 调用 SyncScheduler。

已处理图片哈希使用 Android SharedPreferences 保存最多约 200 条，因此**不升级 Room schema**。

## 7. AI 解析约束

Prompt 强制：

- 先做账单守卫；
- 非账单返回 `[]`；
- 一张截图允许多笔真实独立交易；
- 优惠、红包、原价、券、小计不能形成额外账单；
- 金额统一为正整数 `amountCents`；
- `type` 仅允许 `expense / income / transfer / refund / fee`；
- transfer 使用 `fromAccount / toAccount`；
- 当前本地账户和分类作为候选上下文；
- 不允许模型直接返回 durable account/category ID。

Parser 负责：

- Markdown fence；
- JSON 数组/单对象；
- trailing comma；
- amount/type/currency/time/confidence 硬校验；
- 非法记录丢弃。

## 8. 本地业务映射与去重

`BillCreationService`：

- AI 账户名 -> 本地 account ID：完全匹配、包含匹配、尾四位、微信/支付宝类型 fallback；
- AI 分类名 -> 本地 category ID：完全匹配、包含匹配、现有 `CategoryClassifier` fallback；
- 高置信度进入 `provisional`，其他进入 `candidate`；
- 两者都进入现有待确认箱；
- 发现相同 `externalTransactionId` 时跳过重复创建；
- 若已有通知 candidate 与截图账单在 5 分钟内金额相同，截图账单创建成功后把通知 candidate 标为 ignored，避免重复统计。

## 9. Cloud / Desktop / Browser 不改识别能力

本功能不增加：

- `/finance/capture/image` Cloud endpoint；
- Cloud GLM/AI Provider；
- Cloud 图片存储；
- Desktop 图片上传；
- Browser 图片上传；
- OCR 服务；
- AI Key 跨设备同步。

Android 只同步最终正常财务实体，其他端继续使用既有 Finance 页面。

## 10. 隐私

会离开手机的内容：

- 用户主动用于识别的截图会从 Android **直接发送到用户配置的 Vision Provider**；
- 结构化后的账单通过 LifeTrace Sync 上传 Cloud。

不会进入 LifeTrace Cloud 的内容：

- 原始截图；
- 图片 Base64；
- Vision API Key；
- Provider 完整原始响应；
- 已处理图片 hash 列表；
- pending-share 本地文件路径。

## 11. 测试门禁

`LifeTrace-finance` 现有 PR CI：

```bash
gradle --no-daemon :core:test
gradle --no-daemon :app:testDebugUnitTest
gradle --no-daemon :app:lintDebug
gradle --no-daemon :app:assembleDebug
gradle --no-daemon :app:assembleRelease
gradle --no-daemon :app:connectedDebugAndroidTest
```

新增单测覆盖 bill guard、JSON 容错、金额/类型/时间/置信度、transfer 账户语义、非账单空数组以及截图文件名/路径检测；Android API 34 instrumentation 额外验证 Vision API Key 可以经 AndroidKeyStore 加密往返。

`LifeTrace` 主仓库只有文档边界更新，不新增业务代码；合并前仍检查分支 diff 确保没有 Cloud Vision 残留。

## 12. 完成与合并

1. Android 功能代码完成；
2. 两仓库文档更新；
3. `LifeTrace-finance` PR CI 全绿；
4. 最终 diff 审查：无 OCR、无 Cloud Vision；
5. `LifeTrace-finance` 合并 `main`；
6. `LifeTrace` 文档分支同步 `main`。
