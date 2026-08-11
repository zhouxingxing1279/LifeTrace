# LifeTrace Desktop UI Preview

UI Preview 用于只审查和修改 LifeTrace 桌面应用前端，不启动 Tauri、Rust、SQLite、照片同步服务或云端服务。

它直接渲染正式的 `DesktopApp` / `HengXuShell` 和正式 CSS，只把后端与原生 API 替换为 Mock 数据。

## 本地运行

```bash
cd LifeTrace
git switch ui-review
npm --prefix apps/desktop ci
npm run ui:dev
```

浏览器打开：

```text
http://127.0.0.1:1420
```

可以使用 `view` 参数直接打开页面，例如：

```text
http://127.0.0.1:1420/?view=notes
http://127.0.0.1:1420/?view=transactions
http://127.0.0.1:1420/?view=execution
http://127.0.0.1:1420/?view=analytics
http://127.0.0.1:1420/?view=assistant
```

## 构建静态 Preview

```bash
npm run ui:build
```

输出目录：

```text
apps/desktop/dist-ui-preview
```

该目录可以直接部署到 GitHub Pages、Cloudflare Pages、Vercel 或任意静态站点托管服务。

Cloudflare Pages 推荐配置：

- Branch: `ui-review`
- Build command: `npm --prefix apps/desktop ci && npm run ui:build`
- Output directory: `apps/desktop/dist-ui-preview`
- 环境变量 `LIFETRACE_UI_BASE=/`

## GitHub Pages

仓库已经包含 `.github/workflows/ui-preview-pages.yml`。

第一次使用需要在 GitHub 仓库中打开：

`Settings -> Pages -> Build and deployment -> Source -> GitHub Actions`

之后每次向 `ui-review` push 前端相关代码，Actions 会自动构建并部署 UI Preview。

公共仓库默认地址通常为：

```text
https://zhouxingxing1279.github.io/LifeTrace/
```

## 与正式桌面应用的关系

正式模式：

```text
DesktopApp -> Tauri/Rust -> SQLite / 本地能力 / 云服务
```

Preview 模式：

```text
DesktopApp -> UI Preview Mock Runtime
```

两种模式复用同一套 React 组件和 CSS。不要在 Preview 中复制一套独立页面。

## 日常 UI 审查流程

1. ChatGPT / Codex 修改 `ui-review` 分支中的正式 TSX/CSS。
2. Push 后等待 GitHub Pages / Cloudflare Pages 构建。
3. 打开固定 Preview 地址审查真实 UI。
4. 截图并指出问题。
5. 继续修改，满意后再把 `ui-review` 合并到 `main`。
