# EPIC-03 认证边界

## 本 Epic 定义

```text
AuthProvider trait
AuthenticatedPrincipal (userId, deviceId, appId)
DevelopmentAuthProvider
TestAuthProvider
```

## 规则

- 数据同步接口必须认证（`Authorization: Bearer <token>`）。
- `GET /health/live`、`/health/ready`、`/api/v1/meta/version`、`/api/v1/sync/capabilities` 无需认证。
- 禁止使用 `X-User-Id` 类头指定任意用户。
- `LIFETRACE_ENV=production` 时 `DEV_AUTH_ENABLED=true` → 启动失败。
- 生产环境缺少签名密钥 → 启动失败。
- Token 比较使用常量时间；Token 不进入日志。
- 客户端无法设置 `serverVersion` / `serverModifiedAt`（Payload 经类型化校验）。

## EPIC-04 接管

注册 / 登录 / 密码哈希 / Access+Refresh Token / 撤销 / 设备注册，替换 `AuthProvider` 时同步业务层不变。
