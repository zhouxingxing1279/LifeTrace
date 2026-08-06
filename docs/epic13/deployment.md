# EPIC-13 Web / PWA 部署与回滚

## 1. 构建

```bash
npm ci
npm run lint
npm run test:unit
npm run pwa:build
```

静态产物位于 `dist-web/`。GitHub Actions 会上传名为 `lifetrace-web-pwa` 的短期 artifact。

## 2. 推荐拓扑

```text
Browser / Installed PWA
        |
      HTTPS
        |
Reverse proxy / CDN
  ├── /          -> dist-web 静态文件
  └── /api/*     -> lifetrace-cloud
```

必须使用同源 `/api`，避免 Web Session Cookie 的跨站限制并降低 CORS/CSRF 配置复杂度。

## 3. Nginx 示例

```nginx
server {
    listen 443 ssl http2;
    server_name lifetrace.example.com;

    root /srv/lifetrace/dist-web;
    index index.html;

    location /api/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto https;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    location = /sw.js {
        add_header Cache-Control "no-cache";
        try_files $uri =404;
    }

    location = /manifest.webmanifest {
        add_header Content-Type application/manifest+json;
        add_header Cache-Control "no-cache";
        try_files $uri =404;
    }

    location /assets/ {
        add_header Cache-Control "public, max-age=31536000, immutable";
        try_files $uri =404;
    }

    location / {
        add_header Cache-Control "no-cache";
        try_files $uri /index.html;
    }
}
```

## 4. 环境

本地开发：

```bash
LIFETRACE_CLOUD_URL=http://127.0.0.1:8080 npm run pwa:dev
```

该变量只配置 Vite 开发代理；生产产物始终请求同源 `/api`，不在前端包中嵌入后端密钥或令牌。

## 5. 发布检查

1. `dist-web/index.html`、`sw.js`、manifest、192/512 图标存在；
2. `/api/v1/web/session` 同源可达；
3. HTTPS 有效；
4. `sw.js` 返回 `no-cache`；
5. hash 资源长期缓存；
6. 登录、创建笔记、同步、退出冒烟通过；
7. Lighthouse/浏览器 Performance 面板确认 LCP 目标；
8. PWA 安装提示和离线启动通过。

## 6. 渐进发布

- 将每次构建发布到不可变版本目录，例如 `/releases/<commit-sha>/`；
- `current` 符号链接切换到目标版本；
- 先内部账户验证，再扩大流量；
- 监控登录失败率、sync 4xx/5xx、Service Worker 错误和前端异常。

## 7. 回滚

静态前端回滚不需要数据库变更：

1. 将 `current` 切回上一已验证版本；
2. 保持旧 `sw.js` 可访问，并通过新的 cache name 触发清理；
3. 如 PR 已合入但需代码回退，创建 revert PR，不 force-push `main`；
4. 回滚后重复登录、同步、退出冒烟。

本 EPIC 未修改数据库 Schema，也未改变 sync v1 后端协议，因此回滚不会产生迁移逆操作。
