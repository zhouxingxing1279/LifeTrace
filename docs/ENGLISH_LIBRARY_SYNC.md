# 每日英语文章库同步

## 架构

同步分为三层：

1. `EnglishContentSource` 定义来源统一接口：最新条目、历史条目、详情、标准化和健康检查。
2. 本机 FastAPI 服务负责礼貌访问 VOA、分页发现和正文解析；每篇失败独立记录。
3. Vinext 服务负责 D1 去重、质量判断、任务状态、日志和兼容的 `EnglishArticle` 写入。

文章抓取不调用 DeepSeek。通过基础质量检查的文章直接成为 `READY`，同时写入
`english_processing_queue` 的 `AI_ENRICHMENT` 待办；AI 暂时不可用不会丢失正文。

## 默认来源和库存

默认启用五个 VOA 来源，每个来源的 `initial_fetch_limit` 为 100：

- Science & Technology
- Health & Lifestyle
- Words and Their Stories
- Everyday Grammar
- Education

首次回填目标约 500 篇。它由开发阶段的一次性脚本执行，成功写入并核对后删除脚本。
进度、游标和已写入文章都持久化；脚本中断后再次执行会跳过已存在内容并继续。
完成状态保存在 `english_library_state`，日常应用运行不再触发历史回填。

## 调度

Electron 启动后只执行同步过期检查，不负责创建约 500 篇的历史库存。持续运行时：

- 每 24 小时：增量同步。
- 每 7 天：最近 50 篇重叠补漏。
- 每 30 天：来源、解析成功率、重复、异常短文和音频样本健康检查。

`english_sync_tasks_single_running` 唯一索引保证同一时刻只有一个任务。超过 30 分钟没有
更新的运行任务会被视为应用中断并释放；新回填根据现有文章和来源游标续跑。

## 配置

可通过环境变量调整：

```text
ENGLISH_INITIAL_FETCH_LIMIT=100
ENGLISH_SYNC_INTERVAL_SECONDS=86400
ENGLISH_SYNC_OVERLAP_DAYS=14
ENGLISH_SYNC_RECENT_SCAN_LIMIT=30
ENGLISH_SYNC_WEEKLY_SCAN_LIMIT=50
ENGLISH_SYNC_STALE_AFTER_DAYS=30
ENGLISH_SYNC_ERROR_THRESHOLD=3
ENGLISH_ARTICLE_MIN_WORDS=200
ENGLISH_ARTICLE_MAX_WORDS=3000
ENGLISH_ARTICLE_MIN_ENGLISH_RATIO=0.75
ENGLISH_ARTICLE_MIN_QUALITY_SCORE=60
VOA_SERVICE_TIMEOUT_MS=180000
```

每个来源的抓取数量、扫描数量、重叠天数、请求间隔和启用状态也保存在
`english_content_sources`，可独立调整。

## 接口

- `GET /api/english/sources`
- `PATCH /api/english/sources/:sourceKey`
- `POST /api/english/sources/:sourceKey/sync`
- `POST /api/english/sync`
- `POST /api/english/sync/backfill`
- `POST /api/english/sync/retry-failed`
- `POST /api/english/sync/repair`（`deep: true` 执行月度健康检查）
- `GET /api/english/sync/status`
- `GET /api/english/sync/logs`
- `GET /api/english/articles/stats`

创建任务的接口返回 HTTP 202 和 `taskId`。前端轮询 status，不让长任务阻塞页面。

## 测试与手动验证

```powershell
npm.cmd run test:sync
.\.venv-xunji\Scripts\python.exe -m pytest xunji_service/tests -q
npm.cmd run build
npm.cmd run lint
```

真实网络小规模验证：

```powershell
.\.venv-xunji\Scripts\python.exe scripts\fetch_voa_articles.py `
  --category science --source-key voa_science --mode history `
  --limit 2 --delay 0.2 --output "$env:TEMP\lifetrace-voa-smoke.json"
```

`POST /api/english/sync/backfill` 保留为底层恢复能力，不在 Electron 启动流程或普通管理页面中自动调用。
