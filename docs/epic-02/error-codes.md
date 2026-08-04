# LifeTrace EPIC-02 错误码

> 错误码一旦发布**不得改变含义**。实现：`crates/lifetrace-contracts/src/error.rs`；未知错误码以字符串保留。

## 1. 统一错误体 `ApiErrorV1`

```json
{
  "code": "LIFETRACE_SNAPSHOT_REQUIRED",
  "message": "cursor has expired; snapshot is required",
  "requestId": "req-1",
  "retryable": false,
  "fieldErrors": [],
  "details": null
}
```

## 2. 错误码表

| 错误码 | 含义 | HTTP | retryable |
|---|---|---|---|
| LIFETRACE_PROTOCOL_UNSUPPORTED | 协议版本不受支持 | 426 | false |
| LIFETRACE_SCHEMA_UNSUPPORTED | schema 版本不受支持 | 400 | false |
| LIFETRACE_CLIENT_TOO_OLD | 客户端版本过旧 | 426 | false |
| LIFETRACE_APP_ID_UNSUPPORTED | appId 不受支持 | 400 | false |
| LIFETRACE_AUTH_REQUIRED | 未认证 | 401 | false |
| LIFETRACE_AUTH_INVALID | 认证无效 | 401 | false |
| LIFETRACE_DEVICE_NOT_REGISTERED | 设备未注册 | 403 | false |
| LIFETRACE_DEVICE_REVOKED | 设备已吊销 | 403 | false |
| LIFETRACE_INVALID_REQUEST | 请求结构错误 | 400 | false |
| LIFETRACE_BATCH_TOO_LARGE | 批次超限 | 400 | false |
| LIFETRACE_PAYLOAD_TOO_LARGE | 请求体/单个 payload 超限 | 413 | false |
| LIFETRACE_UNKNOWN_ENTITY_TYPE | 未知实体类型 | 400 | false |
| LIFETRACE_INVALID_ENTITY_PAYLOAD | payload 校验失败 | 400 | false |
| LIFETRACE_DEPENDENCY_MISSING | 依赖实体缺失 | 400 | false |
| LIFETRACE_CHANGE_ID_REUSE | 同一 changeId 使用不同 payload | 400 | false |
| LIFETRACE_BASE_VERSION_MISMATCH | baseServerVersion 不匹配 | 200* | false |
| LIFETRACE_CURSOR_INVALID | cursor 无效 | 400 | false |
| LIFETRACE_CURSOR_EXPIRED | cursor 过期，需 Snapshot | 410 | false |
| LIFETRACE_SNAPSHOT_REQUIRED | 必须先从 Snapshot 开始 | 400 | false |
| LIFETRACE_ATOMIC_GROUP_FAILED | Atomic group 全部失败 | 200* | false |
| LIFETRACE_RATE_LIMITED | 限流 | 429 | true |
| LIFETRACE_TEMPORARILY_UNAVAILABLE | 暂时不可用 | 503 | true |
| LIFETRACE_INTERNAL_ERROR | 服务内部错误 | 500 | true |

*`BASE_VERSION_MISMATCH` 与 `ATOMIC_GROUP_FAILED` 通常作为 Push 单条结果的 `rejected`/`conflict` 返回（HTTP 200），不使用 409 中断整批。

## 3. 稳定约束

- 已发布错误码 wire 字符串不得改变。
- 新错误码只增不改；未知码不得导致整批解析失败。
