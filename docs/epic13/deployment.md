# EPIC-13 Web / PWA 部署与回滚

## 1. 构建

```bash
npm ci
npm run lint
npm run test:unit
npm run web:build
npm run pwa:build
```

PWA 静态产物位于 `dist-web/`。GitHub Actions 上传 `lifetrace-web-pwa` artifact，并验证 manifest、Service Worker、图标和 5 MB 体积门禁。

### 本地开发

LifeTrace Cloud 默认监听 `http://127.0.0.1:8787`，Web 开发服务器默认监听 `http://127.0.0.1:4173`。启动 Web/PWA 开发服务器时，Vite 会将浏览器的 `/api/*` 请求代理到 8787：

```bash
npm run pwa:dev
```

只有后端使用其他地址时才需要覆盖代理目标：

```bash
LIFETRACE_CLOUD_URL=http://127.0.0.1:9000 npm run pwa:dev
```

Windows PowerShell 可使用：

```powershell
$env:LIFETRACE_CLOUD_URL = "http://127.0.0.1:9000"
npm run pwa:dev
```

## 2. 推荐拓扑

```text
Browser / Installed PWA
        |
      HTTPS
        |
Reverse proxy / CDN
        |-- /assets/*  -> dist-web static assets
        |-- /*         -> dist-web/index.html (SPA fallback)
        `-- /api/*     -> LifeTrace Cloud Axum service
                              |
                          PostgreSQL
```

Web 与 API 必须使用同一站点来源，确保 HttpOnly Cookie、SameSite 和 CSRF Origin 校验行为稳定。开发环境由 `vite.web.config.ts` 将 `/api` 代理到 `LIFETRACE_CLOUD_URL`；未设置时使用 `http://127.0.0.1:8787`。

## 3. 在线要求

LifeTrace Web 是云端直写客户端：

- 页面加载、会话恢复和业务操作均需要网络；
- 浏览器不保存业务实体、草稿或待同步队列；
- Service Worker 不缓存页面、静态资源或 API；
- 断网时禁止写操作并提示“数据未保存”；
- 重新联网后刷新页面或点击“刷新云端”恢复最新数据。

部署层不得配置 HTML 的离线 fallback，也不得由 CDN 缓存带身份信息的 `/api` 响应。

## 4. 静态资源策略

推荐响应头：

```text
/index.html
  Cache-Control: no-cache

/sw.js
  Cache-Control: no-cache
  Service-Worker-Allowed: /

/manifest.webmanifest
  Cache-Control: no-cache

/assets/<hash>.*
  Cache-Control: public, max-age=31536000, immutable

/api/*
  Cache-Control: no-store
```

虽然静态 hash 资源可由 HTTP 缓存优化，但 Service Worker 不主动创建 Cache Storage 副本。

## 5. Cookie 与安全配置

生产环境要求：

- 全站 HTTPS；
- Session Cookie：`HttpOnly; Secure`；
- SameSite 根据同源部署策略配置；
- Axum 允许的 Web Origin 必须是实际生产域名；
- 反向代理保留 `Origin`、`Host`、`X-Forwarded-For` 和协议头；
- `/api/v1/sync/push` 等 Browser 写请求校验 `x-csrf-token`；
- 禁止把 access token 或 refresh token注入前端环境变量。

## 6. SPA 路由

以下路径必须回退到 `index.html`：

- `/search`
- `/devices`
- `/finance/*`
- `/notes`
- `/english/*`

`/api/*`、`/sw.js`、`/manifest.webmanifest` 和实际静态资源不得进入 SPA fallback。

## 7. 健康检查

部署后验证：

```bash
curl -I https://<host>/
curl -I https://<host>/manifest.webmanifest
curl -I https://<host>/sw.js
curl -I https://<host>/api/v1/health/live
curl -I https://<host>/api/v1/health/ready
```

浏览器联调必须额外验证：登录、snapshot、push、pull、退出、CSRF 拒绝和跨用户数据隔离。

## 8. 附件能力

当前服务未提供 EPIC-12 对象存储签名上传路由。不要仅部署 `file.metadata` 同步后就开放附件按钮。上线附件前必须同时具备：

- 用户和领域所有权校验；
- 文件大小与 MIME 白名单；
- 签名上传 URL；
- 上传完成确认；
- 下载授权；
- 孤立文件清理与失败重试。

## 9. 发布步骤

1. PR 所有检查通过；
2. squash merge 到 `main`；
3. 使用 `main` 对应 SHA 构建 `dist-web`；
4. 上传到静态站点或 CDN；
5. 原子切换静态版本；
6. 验证健康检查和登录流程；
7. 验证 Service Worker 更新提示；
8. 验证手机、平板和桌面布局。

## 10. 回滚

Web 静态资源回滚：

1. 将站点指向上一个通过 CI 的 `dist-web` artifact；
2. 保持 API 和数据库不变；
3. 确认旧前端仍满足当前最低客户端版本；
4. 浏览器重新加载后获取旧静态版本。

代码回滚：

```bash
git revert <epic13-merge-commit>
```

本 EPIC 不增加数据库迁移；回滚前端不会删除云端业务数据。若 Browser sync 后端改动需要回滚，应同步回滚对应 Rust 路由和集成测试。
