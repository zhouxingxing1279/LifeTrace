# LifeTrace 浏览器前端

浏览器前端是 LifeTrace Cloud 的在线客户端，不是 PWA，也不是桌面应用的本地数据库副本。

## 功能范围

浏览器端保留桌面应用中所有适合云端运行的业务模块：

- 总览与跨模块时间线
- AI 管家
- 坚持项目与打卡
- 每日英语、阅读总结、高亮、生词和统计
- 健身训练、训练文件导入和训练笔记
- 笔记、文件夹、标签与搜索
- 生活日历与每日复盘
- 财务概览、账户、分类、预算、账单和批量导入
- 设备、会话、账号安全、主题和 JSON 云端备份

以下能力只保留在桌面应用，不进入浏览器包：

- 照片同步与媒体库
- 本地加密相册
- 手机局域网上传桥接
- 本地证书、密钥和设备文件路径

浏览器端没有 Service Worker、Web App Manifest、IndexedDB 业务数据库或离线写入队列。

## 架构

```text
web-client React UI
        |
        | HttpOnly Cookie + CSRF
        v
LifeTrace Cloud :8787
        |
        +-- PostgreSQL cloud entities
        +-- browser account/device/session endpoints
        +-- server-side AI proxy
```

所有新增、编辑、删除和导入操作必须收到服务器确认后才更新页面状态。

## 本地启动

启动 PostgreSQL 与云端服务：

```powershell
docker compose -f deploy/cloud/docker-compose.local.yml --profile cloud up -d --build
```

启动浏览器前端：

```powershell
npm.cmd install
npm.cmd run browser:dev
```

访问：

```text
http://127.0.0.1:4173
```

默认云端地址根据页面主机解析为 `http://<当前主机>:8787`。可在构建或启动前设置：

```powershell
$env:VITE_LIFETRACE_CLOUD_URL="http://127.0.0.1:8787"
```

## AI 管家

DeepSeek 密钥只配置在云端服务中：

```powershell
$env:DEEPSEEK_API_KEY="..."
$env:DEEPSEEK_BASE_URL="https://api.deepseek.com"
$env:DEEPSEEK_MODEL="deepseek-chat"
```

未配置密钥或上游暂不可用时，服务端返回基于真实记录数量的本地分析，不会把密钥或浏览器请求直接转发给第三方。

## 构建与测试

```powershell
npm.cmd run lint
npm.cmd run test:unit
npm.cmd run browser:build
cargo test --manifest-path services/lifetrace-cloud/Cargo.toml -- --test-threads=1
```

浏览器产物位于：

```text
dist-browser/
```

Pull Request 必须通过 Browser Web、EPIC-03 PostgreSQL 以及被改动路径触发的既有回归检查后才能合并。

部署静态文件时，需要允许页面访问 LifeTrace Cloud，并把页面的精确 Origin 加入 `CORS_ALLOWED_ORIGINS`。生产环境必须使用 HTTPS 和安全 Cookie。
