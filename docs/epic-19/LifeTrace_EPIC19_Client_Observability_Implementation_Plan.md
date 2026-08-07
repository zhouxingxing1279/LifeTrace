# LifeTrace EPIC-19：客户端可观测性、日志与诊断完整执行方案

> 文档状态：待实施  
> 更新日期：2026-08-07  
> 适用范围：LifeTrace Windows 客户端、Web/PWA、Android 客户端、云端 API 与同步服务  
> Roadmap 对应：EPIC-18 测试、CI/CD 与发布；EPIC-19 监控、日志与运维

---

## 1. 背景与问题定义

LifeTrace 曾出现以下故障：

```text
应用代码将浏览器原生 window.fetch 保存为类成员；
调用时使用 this.fetcher(...)；
Chromium 将调用对象绑定为 API 类实例，而不是 window；
fetch 在真正创建网络请求前抛出 TypeError：
Can only call window.fetch on instance of Window
```

上层代码捕获异常后统一包装为“无法连接 LifeTrace 云端”，造成以下误导：

- 登录按钮点击后看起来没有反应；
- Chromium Network 面板中没有请求；
- 服务端没有访问日志；
- 排查方向被错误引导到端口、CORS、服务器状态和数据库；
- 原始错误名称、消息、调用栈和发生阶段丢失；
- 普通用户提示与开发诊断信息没有分离。

这个案例说明，只有服务端日志、同步日志和 request ID 不够。LifeTrace 必须建立从用户操作到客户端运行时、API 调用阶段、网络请求、服务端处理和业务写入的端到端可观测链路。

---

## 2. 建设目标

本方案需要实现以下结果：

1. 请求发出前的客户端编程错误可被记录。
2. 请求发出后的网络错误、HTTP 错误、解析错误可明确区分。
3. 用户看到友好提示时，原始错误仍完整保留。
4. Windows 安装版无需打开 DevTools，也能导出诊断日志。
5. 每次用户操作、客户端请求与服务端请求可以关联。
6. 日志不泄露密码、Token、Cookie、健康正文、账单正文或加密相册信息。
7. 类似 `window.fetch` 绑定错误必须有自动回归测试。
8. 日志系统失败不能反向导致业务功能崩溃。

### 2.1 非目标

第一阶段不建设复杂商业 APM 平台，不强依赖 Sentry、Datadog 或 Elastic，也不自动上传用户完整日志。先完成本地优先、可导出、可检索、可关联的基础能力。

---

## 3. 总体架构

```text
用户操作
  ↓ operationId
React / Web Renderer
  ├─ Global Error Handler
  ├─ unhandledrejection
  ├─ React ErrorBoundary
  ├─ API Client Instrumentation
  └─ Structured Logger
        ↓ 安全桥接
Windows Runtime Adapter
  ├─ Electron IPC（当前运行时，如仍使用 Electron）
  └─ Tauri Command / Plugin（迁移后）
        ↓
本地日志写入器
  ├─ app.log
  ├─ api.log
  ├─ error.log
  ├─ sync.log
  └─ audit.log
        ↓
诊断中心
  ├─ 查看最近错误
  ├─ 打开日志目录
  ├─ 复制错误编号
  ├─ 开启详细日志
  └─ 导出脱敏诊断包

客户端 HTTP 请求
  ↓ clientTraceId / X-Client-Trace-Id
LifeTrace Cloud
  ↓ requestId / X-Request-Id
服务端结构化日志、指标与告警
```

### 3.1 关键关联 ID

| 字段 | 生成位置 | 生命周期 | 用途 |
|---|---|---|---|
| `sessionId` | 客户端启动 | 单次应用会话 | 关联一次启动期间的日志 |
| `operationId` | 用户触发业务操作 | 单次登录、保存、同步等操作 | 关联 UI、API 和业务处理 |
| `clientTraceId` | 客户端每次 API 请求 | 单个请求 | 关联请求前后阶段 |
| `requestId` | 服务端入口 | 单个服务端请求 | 关联中间件、业务与数据库日志 |
| `syncBatchId` | 同步批次 | 单次 push/pull | 关联同步队列与服务端处理 |
| `errorId` | 每个 error/fatal 事件 | 单个异常 | 供界面和诊断包引用 |

客户端请求头应加入：

```http
X-Client-Trace-Id: <uuid>
X-Operation-Id: <uuid>
X-Client-Version: <version>
X-Device-Id: <脱敏设备标识>
```

服务端响应应返回：

```http
X-Request-Id: <uuid>
```

---

## 4. 日志事件模型

### 4.1 统一日志结构

```ts
export type LogLevel = 'debug' | 'info' | 'warn' | 'error' | 'fatal';

export type RuntimeName =
  | 'web'
  | 'renderer'
  | 'electron-main'
  | 'tauri'
  | 'android'
  | 'cloud';

export interface LogEvent {
  timestamp: string;
  level: LogLevel;
  event: string;
  runtime: RuntimeName;
  module: string;
  appVersion: string;
  environment: 'development' | 'staging' | 'production';
  sessionId: string;
  operationId?: string;
  clientTraceId?: string;
  requestId?: string;
  syncBatchId?: string;
  errorId?: string;
  stage?: string;
  durationMs?: number;
  data?: Record<string, unknown>;
  error?: SerializedError;
}

export interface SerializedError {
  name: string;
  message: string;
  stack?: string;
  cause?: SerializedError;
  code?: string;
  category?: ErrorCategory;
}
```

### 4.2 命名规范

事件名统一采用：

```text
<domain>.<action>.<state>
```

示例：

```text
app.start.begin
app.start.ready
ui.login.clicked
api.request.start
api.fetch.invoke
api.response.received
api.response.parse_failed
api.request.failed
sync.push.started
sync.push.failed
renderer.uncaught_error
react.render_failed
runtime.preload_failed
runtime.renderer_gone
```

禁止使用无法搜索和聚合的事件名，例如：

```text
出错了
请求失败
发生异常
something wrong
```

---

## 5. 错误分类

```ts
export type ErrorCategory =
  | 'programming'
  | 'validation'
  | 'network'
  | 'timeout'
  | 'aborted'
  | 'http'
  | 'authentication'
  | 'authorization'
  | 'conflict'
  | 'parse'
  | 'storage'
  | 'database'
  | 'sync'
  | 'runtime'
  | 'unknown';
```

### 5.1 分类原则

- `programming`：非法调用、空对象访问、函数不存在、原生方法 this 绑定错误。
- `network`：DNS、连接拒绝、TLS、离线等真正的传输失败。
- `timeout`：由客户端或服务端超时控制器明确产生。
- `http`：已经收到非 2xx HTTP 响应。
- `parse`：响应已收到，但 JSON、日期或业务结构解析失败。
- `authentication`：Token 无效、过期、刷新失败。
- `conflict`：同步版本冲突或幂等冲突。
- `runtime`：preload、渲染进程退出、WebView 崩溃等运行时异常。

不得仅凭 `error instanceof TypeError` 将错误分类为网络故障，因为 `fetch` 非法调用同样是 TypeError。

### 5.2 fetch 绑定错误识别

至少识别以下消息：

```text
Illegal invocation
Can only call window.fetch on instance of Window
Failed to execute 'fetch' on 'Window'
```

识别后记录：

```json
{
  "category": "programming",
  "stage": "fetch.invoke",
  "requestSent": false
}
```

---

## 6. Logger 核心设计

建议目录：

```text
app/src/observability/
├── logger.ts
├── log-types.ts
├── error-serializer.ts
├── error-classifier.ts
├── redaction.ts
├── context.ts
├── global-handlers.ts
├── react-error-boundary.tsx
├── api-instrumentation.ts
├── transports/
│   ├── console-transport.ts
│   ├── browser-buffer-transport.ts
│   ├── electron-ipc-transport.ts
│   └── tauri-transport.ts
└── __tests__/
```

### 6.1 Logger 接口

```ts
export interface Logger {
  debug(event: string, data?: Record<string, unknown>): void;
  info(event: string, data?: Record<string, unknown>): void;
  warn(event: string, data?: Record<string, unknown>): void;
  error(
    event: string,
    error: unknown,
    data?: Record<string, unknown>,
  ): string;
  fatal(
    event: string,
    error: unknown,
    data?: Record<string, unknown>,
  ): string;
  child(context: Record<string, unknown>): Logger;
}
```

`error()` 和 `fatal()` 返回 `errorId`，界面可显示：

```text
登录请求未能完成。
错误编号：ERR-20260807-8F3A
```

### 6.2 Logger 自身容错

- 所有 transport 写入都必须被内部 try/catch 隔离。
- 一个 transport 失败不能影响其他 transport。
- Logger 不得抛出异常给业务调用方。
- 序列化循环引用时退化为安全字符串。
- 单条日志设置大小上限，超出后截断并记录 `truncated: true`。

---

## 7. 客户端 API 调用改造

### 7.1 Fetch 类型与默认实现

不要保存未绑定的原生方法：

```ts
// 错误
this.fetcher = window.fetch;
```

使用显式包装：

```ts
export type Fetcher = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export const browserFetcher: Fetcher = (input, init) =>
  window.fetch(input, init);
```

或：

```ts
export const browserFetcher = window.fetch.bind(window);
```

推荐第一种，调用上下文更明确，也便于测试和替换。

### 7.2 请求阶段状态机

```text
created
→ request.start
→ request.prepare
→ fetch.invoke
→ request.sent
→ response.received
→ response.read
→ response.parse
→ request.succeeded
```

失败时必须记录最后成功阶段：

```text
created             参数尚未构造完成
request.prepare     Headers、body 或 URL 构造失败
fetch.invoke        调用 fetch 时立即失败，请求可能尚未发送
request.sent        网络传输中失败
response.received   已有 HTTP 响应
response.read       读取响应体失败
response.parse      解析响应体失败
```

### 7.3 Instrumented API Client

```ts
export class ApiClient {
  constructor(
    private readonly baseUrl: string,
    private readonly fetcher: Fetcher = browserFetcher,
    private readonly logger: Logger,
  ) {}

  async request<T>(
    path: string,
    init: RequestInit = {},
    context: { operationId?: string; action?: string } = {},
  ): Promise<T> {
    const clientTraceId = crypto.randomUUID();
    const startedAt = performance.now();
    const method = init.method ?? 'GET';
    const url = new URL(path, this.baseUrl).toString();
    let stage = 'request.prepare';
    let requestSent = false;

    const log = this.logger.child({
      module: 'api-client',
      operationId: context.operationId,
      clientTraceId,
      action: context.action,
      method,
      url: sanitizeUrl(url),
    });

    log.info('api.request.start');

    try {
      const headers = buildSafeHeaders(init.headers, {
        'X-Client-Trace-Id': clientTraceId,
        'X-Operation-Id': context.operationId,
      });

      stage = 'fetch.invoke';
      log.debug('api.fetch.invoke');

      const responsePromise = this.fetcher(url, {
        ...init,
        headers,
      });

      requestSent = true;
      stage = 'request.sent';
      const response = await responsePromise;

      stage = 'response.received';
      const requestId = response.headers.get('X-Request-Id') ?? undefined;

      log.info('api.response.received', {
        status: response.status,
        requestId,
      });

      stage = 'response.read';
      const text = await response.text();

      if (!response.ok) {
        throw new HttpError(response.status, text, requestId);
      }

      stage = 'response.parse';
      const result = text ? (JSON.parse(text) as T) : (undefined as T);

      log.info('api.request.succeeded', {
        status: response.status,
        durationMs: Math.round(performance.now() - startedAt),
      });

      return result;
    } catch (error) {
      const category = classifyError(error, { stage, requestSent });
      const errorId = log.error('api.request.failed', error, {
        stage,
        requestSent,
        category,
        durationMs: Math.round(performance.now() - startedAt),
      });

      throw new LifeTraceRequestError(
        createUserMessage(category),
        {
          cause: error,
          category,
          stage,
          requestSent,
          clientTraceId,
          errorId,
        },
      );
    }
  }
}
```

注意：`requestSent=true` 只能表示 fetch 返回了 Promise 并开始处理，不保证数据已经到达服务器。因此日志字段用于排查阶段，而不是网络协议级证明。

---

## 8. 原始错误链保留

### 8.1 禁止模式

```ts
catch {
  throw new Error('无法连接 LifeTrace 云端');
}
```

```ts
catch (error) {
  throw new Error('无法连接 LifeTrace 云端');
}
```

### 8.2 正确模式

```ts
catch (error) {
  const errorId = logger.error('cloud.request.failed', error, {
    stage,
    operationId,
  });

  throw new LifeTraceClientError(
    '请求未能完成',
    {
      cause: error,
      errorId,
      category: classifyError(error),
    },
  );
}
```

### 8.3 Error 序列化

序列化必须递归保留 `cause`，但限制最大深度，例如 5 层，防止无限链或超大日志。

```ts
export function serializeError(
  value: unknown,
  depth = 0,
): SerializedError {
  if (depth > 5) {
    return { name: 'Error', message: '[cause depth exceeded]' };
  }

  if (value instanceof Error) {
    return {
      name: value.name,
      message: value.message,
      stack: value.stack,
      cause: value.cause
        ? serializeError(value.cause, depth + 1)
        : undefined,
    };
  }

  return {
    name: typeof value,
    message: safeStringify(value),
  };
}
```

---

## 9. 全局前端异常捕获

### 9.1 window.error

```ts
window.addEventListener('error', event => {
  logger.error('renderer.uncaught_error', event.error ?? event.message, {
    filename: event.filename,
    line: event.lineno,
    column: event.colno,
  });
});
```

### 9.2 unhandledrejection

```ts
window.addEventListener('unhandledrejection', event => {
  logger.error('renderer.unhandled_rejection', event.reason);
});
```

### 9.3 React ErrorBoundary

需要记录：

- Error 对象；
- React component stack；
- 当前路由；
- 当前 operationId；
- 应用版本；
- 最近 20 条安全 breadcrumb。

ErrorBoundary 只捕获 React 渲染生命周期错误，不替代 `window.error` 和 `unhandledrejection`。

### 9.4 Breadcrumb

可记录有限的、脱敏后的操作轨迹：

```text
route.changed
ui.login.clicked
api.request.start
api.fetch.invoke
```

禁止记录输入框原文、密码、笔记正文、邮件正文或账单明细。

---

## 10. Windows 运行时日志桥接

### 10.1 Electron 当前实现

Renderer 不直接访问文件系统，通过 preload 暴露最小接口：

```ts
contextBridge.exposeInMainWorld('diagnostics', {
  writeLog: (event: LogEvent) =>
    ipcRenderer.send('diagnostics:write-log', event),
  getStatus: () =>
    ipcRenderer.invoke('diagnostics:get-status'),
  openLogDirectory: () =>
    ipcRenderer.invoke('diagnostics:open-log-directory'),
  exportBundle: () =>
    ipcRenderer.invoke('diagnostics:export-bundle'),
});
```

主进程还应监听：

```text
console-message
preload-error
did-fail-load
render-process-gone
unresponsive
child-process-gone
```

### 10.2 Tauri 迁移适配

Logger 核心、日志模型、脱敏、错误分类和 API instrumentation 保持不变，仅将 transport 替换为 Tauri command 或日志插件。业务层不得直接依赖 Electron API。

### 10.3 Web/PWA

Web 端第一阶段使用：

- 开发环境 Console transport；
- 内存环形缓冲区；
- IndexedDB 保存最近错误摘要；
- 用户主动导出 JSON；
- 后续再评估主动上传错误摘要。

---

## 11. 本地日志文件设计

建议目录：

```text
<appLogs>/LifeTrace/
├── app.log
├── api.log
├── sync.log
├── audit.log
├── error.log
├── network/
└── archive/
```

### 11.1 分流规则

| 文件 | 内容 |
|---|---|
| `app.log` | 启动、退出、页面生命周期、配置加载 |
| `api.log` | 请求阶段、响应状态、耗时、错误分类 |
| `sync.log` | outbox、push、pull、冲突、重试 |
| `audit.log` | 用户和 AI 的关键写操作摘要 |
| `error.log` | error/fatal 的完整脱敏事件 |

### 11.2 轮转与保留

第一阶段建议：

- 单文件最大 10 MB；
- 每种日志保留最多 10 个轮转文件；
- 普通日志保留 14 天；
- error 日志保留 30 天；
- 网络诊断日志最多 20 MB，用户关闭诊断后停止写入；
- 应用启动时异步清理过期文件。

### 11.3 写入格式

使用 JSON Lines：每行一个完整 JSON 对象，便于 grep、脚本和诊断工具解析。

---

## 12. 隐私与日志脱敏

### 12.1 永不记录

- 明文密码；
- Access Token、Refresh Token；
- Cookie 和 `Authorization`；
- 邮箱授权码；
- 完整银行卡号；
- 笔记、日记、邮件和健康档案正文；
- 加密相册文件名、绝对路径和密钥；
- AI API Key；
- 文件原始二进制内容。

### 12.2 默认脱敏字段

```text
password
passcode
secret
token
accessToken
refreshToken
authorization
cookie
apiKey
emailAuthorizationCode
```

字段名匹配应忽略大小写，并递归处理对象和数组。

### 12.3 URL 脱敏

日志默认只保留：

```text
origin + pathname
```

查询参数使用白名单。未知查询参数不写入，避免分享链接、Token 或用户输入泄露。

### 12.4 请求体策略

默认不记录请求体。只允许业务模块显式提供安全摘要，例如：

```json
{
  "entityType": "transaction",
  "itemCount": 3,
  "payloadBytes": 1240
}
```

---

## 13. 诊断中心设计

设置页新增“诊断与日志”。

### 13.1 状态信息

- LifeTrace 版本和构建号；
- 操作系统和运行时版本；
- 数据库 schema 版本；
- 同步协议版本；
- 云端环境和 API origin；
- 最近启动时间；
- 最近同步时间；
- 最近错误时间和 errorId；
- 当前日志级别；
- 日志目录大小。

### 13.2 操作

- 查看最近错误；
- 复制最近错误摘要；
- 打开日志目录；
- 开启/关闭详细日志；
- 开启/关闭网络诊断；
- 导出诊断包；
- 清理旧日志；
- 开发版本打开 DevTools。

### 13.3 诊断包

```text
LifeTrace-Diagnostics-<timestamp>.zip
├── manifest.json
├── system-info.json
├── recent-errors.json
├── app.log
├── api.log
├── sync.log
├── error.log
└── network/netlog.json（仅用户主动开启时）
```

导出前必须再次执行脱敏扫描，并生成：

```json
{
  "redactionVersion": 1,
  "filesIncluded": [],
  "filesExcluded": [],
  "sensitiveMatchesRemoved": 0
}
```

---

## 14. 服务端可观测性

### 14.1 Axum 中间件

每个请求：

- 读取 `X-Client-Trace-Id` 和 `X-Operation-Id`；
- 生成服务端 `requestId`；
- 将关联 ID 放入 tracing span；
- 响应写回 `X-Request-Id`；
- 记录 method、route template、status、duration；
- 不记录完整 query 和 body。

### 14.2 指标

第一阶段至少包括：

```text
http_requests_total
http_request_duration_seconds
http_errors_total
sync_push_total
sync_pull_total
sync_failures_total
sync_outbox_backlog
db_pool_connections
file_upload_failures_total
backup_last_success_timestamp
```

### 14.3 错误响应

服务端错误响应采用稳定结构：

```json
{
  "error": {
    "code": "AUTH_INVALID_CREDENTIALS",
    "message": "账号或密码不正确",
    "requestId": "..."
  }
}
```

生产环境不返回内部堆栈；内部日志通过 requestId 关联。

---

## 15. Source Map 与构建产物

- 开发环境保留完整 Source Map。
- 生产环境生成 hidden Source Map。
- Source Map 进入私有构建归档，不公开部署给普通用户。
- 每个发布版本保存版本号到 commit SHA 的映射。
- 错误日志必须包含 appVersion、buildNumber 和 commit SHA。
- 发布脚本应验证 Source Map 已生成并归档。

---

## 16. 自动测试方案

### 16.1 单元测试

必须覆盖：

- `serializeError` 保留 name、message、stack、cause；
- 循环引用不会导致 Logger 崩溃；
- 敏感字段被递归脱敏；
- URL 查询参数默认被移除；
- Logger transport 失败不会影响业务；
- `classifyError` 正确区分 programming/network/http/parse；
- `browserFetcher` 始终通过 `window.fetch(...)` 调用；
- 日志大小超限时被安全截断。

### 16.2 fetch 绑定回归测试

```ts
it('默认 fetcher 不依赖 ApiClient 实例的 this', async () => {
  const spy = vi.spyOn(window, 'fetch').mockResolvedValue(
    new Response('{}', {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    }),
  );

  const client = new ApiClient(baseUrl, browserFetcher, logger);
  await client.request('/health');

  expect(spy).toHaveBeenCalledOnce();
});
```

再增加故障注入测试：

```ts
it('fetch 调用前异常记录为 programming 且 requestSent=false', async () => {
  const invalidFetcher = window.fetch;
  const client = new ApiClient(baseUrl, invalidFetcher, logger);

  await expect(client.request('/health')).rejects.toMatchObject({
    category: 'programming',
    stage: 'fetch.invoke',
    requestSent: false,
  });
});
```

说明：不同测试运行时对原生绑定检查可能不同。若 jsdom 不复现 Chromium 行为，需要使用 Playwright/Electron 集成测试在真实 Chromium 中验证。

### 16.3 集成测试

- API 请求前参数构造异常；
- 网络断开；
- DNS/连接拒绝；
- 请求超时；
- HTTP 401、403、409、500；
- 返回非法 JSON；
- 返回空响应；
- React 渲染异常；
- 未处理 Promise；
- preload 失败；
- Renderer 异常退出；
- 日志目录只读；
- 日志文件达到轮转阈值；
- 诊断包脱敏。

### 16.4 E2E 验收场景

复现原故障时必须得到：

```text
UI：显示“登录请求未能完成”，附 errorId
Network：没有请求
api.log：api.request.start
api.log：api.fetch.invoke
error.log：api.request.failed
stage=fetch.invoke
requestSent=false
category=programming
原始 TypeError message 和 stack 完整保留
服务端：没有对应 requestId
```

这正是预期诊断结果，而不是日志系统失败。

---

## 17. CI 门禁

每个 PR 必须执行：

```text
npm run typecheck
npm run lint
npm run test:unit
npm run test:observability
npm run test:e2e:critical
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

增加规则：

- 禁止 `catch { throw new Error('无法连接...') }` 形式丢弃原始异常；
- 禁止将 `window.fetch` 直接赋值给会以对象方法形式调用的字段；
- API Client 必须使用统一 instrumentation；
- 新增 API 调用不得直接散落使用裸 `fetch`；
- error/fatal 日志必须包含 errorId；
- Source Map 归档检查必须通过。

可先通过 ESLint 自定义规则或代码搜索脚本实现，后续再抽取正式插件。

---

## 18. 分阶段实施计划

### Phase 0：故障修复与防回归

目标：立即消除当前 fetch 绑定风险。

任务：

- [ ] 将所有 `window.fetch` 直接引用改为包装函数或 bind；
- [ ] 搜索所有 `this.fetcher(...)`、`fetcher:` 和依赖注入点；
- [ ] 保留原始 `cause`；
- [ ] 增加 Chromium 环境回归测试；
- [ ] 确认登录、同步和上传 API 均走统一 Client。

完成标准：原故障无法再次复现，且测试可以在故意恢复错误实现时失败。

### Phase 1：最小客户端日志基线

目标：安装版可记录前端异常。

任务：

- [ ] 建立日志事件类型；
- [ ] 建立 error serializer；
- [ ] 建立 redaction；
- [ ] 建立 console 与本地文件 transport；
- [ ] 捕获 window.error 和 unhandledrejection；
- [ ] 增加 React ErrorBoundary；
- [ ] 增加主进程/运行时异常监听；
- [ ] 增加日志轮转。

完成标准：关闭 DevTools 后制造错误，仍能在 error.log 中看到原始错误和 Source Map 可解析堆栈。

### Phase 2：API 阶段日志与错误分类

目标：判断请求是否在发送前失败。

任务：

- [ ] 实现 Instrumented ApiClient；
- [ ] 实现阶段状态机；
- [ ] 实现 ErrorCategory；
- [ ] 增加 operationId/clientTraceId；
- [ ] 分离内部错误和用户提示；
- [ ] 接入登录、同步、文件上传和 AI API。

完成标准：任何 API 失败都能回答“发生在哪个阶段、是否收到响应、错误属于哪一类”。

### Phase 3：诊断中心与诊断包

目标：用户无需命令行即可提供有效诊断信息。

任务：

- [ ] 设置页新增诊断中心；
- [ ] 显示最近错误和日志状态；
- [ ] 支持打开日志目录；
- [ ] 支持开启详细日志；
- [ ] 支持导出 ZIP；
- [ ] 导出前执行二次脱敏；
- [ ] 增加诊断包自动测试。

完成标准：用户点击一次即可导出不含敏感信息的诊断包。

### Phase 4：服务端关联与指标

目标：客户端请求与服务端处理可以端到端追踪。

任务：

- [ ] Axum requestId 中间件；
- [ ] 传递 clientTraceId/operationId；
- [ ] 服务端 JSON 日志；
- [ ] API、同步、数据库、文件和备份指标；
- [ ] 核心告警；
- [ ] requestId 查询方式。

完成标准：已到达服务端的请求可从客户端 errorId 追踪到 requestId 和服务端业务错误。

### Phase 5：持续质量与治理

目标：可观测性成为架构约束，而不是一次性功能。

任务：

- [ ] CI 增加日志与错误链门禁；
- [ ] 每次发布执行故障注入测试；
- [ ] 定期审查敏感日志；
- [ ] 记录日志 schema 版本；
- [ ] 建立事件名称登记表；
- [ ] 评估可选的远程错误摘要上报。

完成标准：新增模块默认接入统一 Logger 和 ApiClient，不再重复实现错误包装。

---

## 19. 推荐 Issue 拆分

```text
OBS-001 统一日志事件模型与 Logger 接口
OBS-002 Error serializer 与 cause 链
OBS-003 递归日志脱敏与 URL 清理
OBS-004 Windows 本地日志 transport
OBS-005 全局 window/error/rejection 捕获
OBS-006 React ErrorBoundary 与 breadcrumb
OBS-007 Instrumented ApiClient
OBS-008 API 阶段状态机与错误分类
OBS-009 fetch 绑定错误回归测试
OBS-010 Electron/Tauri 运行时异常桥接
OBS-011 日志轮转与保留策略
OBS-012 诊断中心 UI
OBS-013 脱敏诊断包导出
OBS-014 Axum requestId 与 trace 关联
OBS-015 服务端指标与告警
OBS-016 Source Map 私有归档
OBS-017 CI 可观测性门禁
OBS-018 故障注入与发布验收
```

每个 Issue 必须包含：输入、输出、修改范围、隐私风险、测试和验收证据。

---

## 20. Definition of Done

EPIC-19 的客户端部分只有在以下条件全部满足时才算完成：

- [ ] 未打开 DevTools 时仍能记录 Renderer 原始异常；
- [ ] `window.fetch` 绑定错误被归类为 programming；
- [ ] 请求前错误记录 `stage=fetch.invoke` 和 `requestSent=false`；
- [ ] 用户提示不会覆盖 error.message、stack 和 cause；
- [ ] 所有核心 API 使用统一 ApiClient；
- [ ] 日志可由 sessionId、operationId、clientTraceId、requestId 和 errorId 关联；
- [ ] 日志文件自动轮转且不会无限增长；
- [ ] 诊断包可以在设置页导出；
- [ ] Token、密码、Cookie、邮件正文和个人敏感正文不会进入日志；
- [ ] Source Map 可用于解析生产堆栈；
- [ ] 单元、集成和 Chromium E2E 测试通过；
- [ ] 故意恢复未绑定 fetch 实现时，CI 必须失败；
- [ ] Logger 写入失败不会阻断登录、保存和同步业务；
- [ ] 已到达云端的请求可通过 requestId 追踪；
- [ ] 未到达云端的请求也能通过客户端阶段日志定位。

---

## 21. 实施后的目标日志示例

```json
{
  "timestamp": "2026-08-07T09:30:12.341+08:00",
  "level": "error",
  "event": "api.request.failed",
  "runtime": "renderer",
  "module": "cloud-auth",
  "appVersion": "0.1.0",
  "environment": "production",
  "sessionId": "...",
  "operationId": "...",
  "clientTraceId": "...",
  "errorId": "ERR-20260807-8F3A",
  "stage": "fetch.invoke",
  "durationMs": 1,
  "data": {
    "action": "login",
    "method": "POST",
    "url": "https://cloud.example.com/api/v1/auth/login",
    "requestSent": false,
    "category": "programming"
  },
  "error": {
    "name": "TypeError",
    "message": "Can only call window.fetch on instance of Window",
    "stack": "..."
  }
}
```

看到这条日志后，排查结论应立即是：客户端在调用 fetch 阶段发生编程错误，请求未进入网络层；无需优先排查云端端口、CORS、数据库和服务状态。
