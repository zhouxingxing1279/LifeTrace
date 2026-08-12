# LifeTrace 智能截图记账执行方案

> 目标：参考 BeeCount 的“截图发现 → 账单抽取 → 结构化交易 → 分类/账户匹配 → 入库”思路，在不复制 BeeCount 源码的前提下，将**新增账单能力收敛到 Android**。Desktop、Browser 与 Cloud 不处理原始截图，只消费 Android 同步上来的结构化财务数据。
>
> 实施分支：`feature/smart-bill-capture`（LifeTrace 与 LifeTrace-finance 两仓库同名分支）。

## 1. 最终边界

### Android：唯一新增账单入口

- 系统分享支付截图到 LifeTrace Finance。
- 可选 MediaStore 截图监听。
- 使用 Android 本地 ML Kit 中文 OCR 识别文本。
- 使用本地账单解析器识别金额、商户、交易类型、支付账户、时间和分类提示。
- 结果先写为 `candidate/provisional`，用户确认后成为正式交易。
- 复用现有 Room、证据、Outbox 和 Sync Engine。
- 原始截图不进入 Sync，也不上传 Cloud。

### Cloud：只同步结构化数据

- 不增加图片上传接口。
- 不增加 Vision Provider、GLM API Key 或图片存储。
- 继续通过既有 `finance.transaction` / `finance.transaction_evidence` 同步协议接收 Android 产生的结构化账单。

### Desktop / Browser：查看、修改、统计

- 不增加图片记账入口。
- 不运行 OCR/VLM。
- 继续读取同步后的财务交易，允许已有的编辑、分类、统计和账户管理能力。

## 2. Android 本地识别架构

```text
支付截图 / 系统分享图片
        |
        v
Screenshot Detector / Share Receiver
        |
        v
SmartCaptureCoordinator
        |
        +-- SHA-256 去重
        +-- 截图来源/时间判断
        |
        v
ML Kit Text Recognition v2 (Chinese)
        |
        v
LocalBillParser
        |
        +-- 是否为账单
        +-- amountCents
        +-- expense/income/refund/transfer/fee
        +-- merchant/item
        +-- occurredAt
        +-- accountHint
        +-- categoryHint
        |
        v
本地账户/分类匹配
        |
        v
finance.transaction(status=candidate/provisional)
finance.transaction_evidence(source_type=local_ocr)
smart_capture_events
sync_outbox
        |
        v
待确认箱 -> Sync -> Cloud -> Desktop/Browser
```

## 3. 为什么第一版不直接塞完整本地 VLM

第一版优先采用 **OCR + 领域解析器**：

- 微信/支付宝/银行支付截图的核心字段主要是结构化文字；
- ML Kit 可以完全在设备端识别中文；
- APK/运行内存压力远小于随包或下载数 GB 的通用多模态模型；
- 可测试、可解释，金额不会因为生成模型幻觉而被随意改写；
- 对固定支付平台可持续补充解析规则和 fixture。

后续增强层可选：

- 在兼容设备上接入 Android 设备端 Gemini Nano / ML Kit Prompt API，用于 OCR 后的语义消歧；
- 或引入自有 LiteRT/Gemma 多模态模型。

增强模型只能作为 parser 的补充，最终仍需通过金额、交易类型和时间等硬校验。

## 4. Android 数据模型

复用：

- `finance_transactions`
- `finance_transaction_evidence`
- `sync_outbox`

新增本地表 `smart_capture_events`：

- `id`
- `local_profile_id`
- `image_hash`
- `capture_source` (`share` / `screenshot_monitor`)
- `engine` (`mlkit_ocr_v2`)
- `status`
- `captured_at`
- `transaction_ids_json`
- `error_code`

数据库从 v1 升级到 v2，并提供显式 `MIGRATION_1_2`，不得因升级删除既有财务数据。

## 5. Android 图片入口

### Share Receiver

把现有图片分享占位实现改成真实识别：

1. 缓存共享图片；
2. 传给 `SmartCaptureCoordinator`；
3. OCR + 解析；
4. 写入候选账单；
5. 删除临时图片。

### Screenshot Monitor

- 使用 `ContentObserver + MediaStore.Images`。
- 仅处理最近新增图片。
- 文件名/路径命中 `screenshot`、`截屏`、`截图`、`screen_shot`、`screen shot` 等。
- 使用 SHA-256 + `smart_capture_events` 防重复。
- 用户显式开启后才注册 observer。
- Android 13+ 需要 `READ_MEDIA_IMAGES`；较低版本按系统要求申请媒体读取权限。
- 第一版不使用永久前台服务；进程被系统杀死后不承诺继续监听，系统分享是可靠兜底入口。

## 6. 本地账单解析策略

`LocalBillParser` 必须先做账单守卫，再抽字段：

- 支付成功、交易详情、账单详情、退款、收款等关键词提高账单置信度；
- 普通聊天、文章、设置页、商品详情但未支付等不创建交易；
- 金额只接受明确货币表达，转换为整数分；
- 支出/收入/退款/转账由语义关键词决定；
- 商户优先取“商户/收款方/付款给/商品说明”等邻近文本；
- 支付方式用于本地账户 hint 匹配；
- 无法确认的字段留空，不能猜造；
- 低置信度结果进入 `candidate`，必须人工确认。

## 7. 隐私与安全

- 原图只在 Android 本地处理。
- 不上传截图、OCR 全文或图片 hash 到 Cloud。
- Sync 只同步正常财务实体与最小 evidence 元数据。
- 日志不得保存 OCR 全文或截图路径；仅记录脱敏后的 engine/status/error code。

## 8. 测试

### Android

```bash
gradle :core:test :app:testDebugUnitTest :app:lintDebug :app:assembleDebug
```

覆盖：

- 微信/支付宝/银行卡典型 OCR fixture；
- 非账单 fixture；
- 金额和交易类型解析；
- screenshot path detector；
- SHA-256 去重；
- candidate/evidence/event 同事务写入；
- Room v1 -> v2 migration；
- ShareReceiver 图片路径透传；
- 权限未授予时安全降级。

### LifeTrace 主仓库

无需新增 Vision 逻辑，只验证现有同步/财务展示没有回归：

```bash
npm run test:desktop
npm run test:rust
npm run test:cloud
npm run contracts:check
npm run browser:build
```

## 9. 验收标准

- Android 分享微信/支付宝支付截图，可在**无 Vision 云服务**的情况下生成候选账单。
- 开启截图监听并授予权限后，新截图可在 App 进程存活期间自动识别；关闭后不监听。
- 非账单截图不会创建交易。
- 原始截图从不进入 Cloud/Sync。
- Android 确认后的交易能照常同步到 Cloud，并在 Desktop/Browser 中查看。
- Desktop/Browser 不增加图片记账入口。
- 所有改动仅停留在 `feature/smart-bill-capture` / PR，不直接合并 `main`。
