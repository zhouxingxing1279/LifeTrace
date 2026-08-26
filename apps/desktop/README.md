# LifeTrace Desktop

LifeTrace 的 Windows 桌面端基于 Tauri 2 + WebView2，前端使用 React/Vite，后端使用 Rust + SQLite。

## 启动行为

桌面窗口创建后会先由静态 HTML 渲染启动界面，再进入 React/Tauri 初始化流程。窗口状态、主题、桥接层和本地 SQLite 服务初始化均受统一错误兜底保护；任何启动异常都应显示“LifeTrace 启动失败”和具体错误，而不是留下空白 WebView。

Windows 构建使用 ES2020 作为桌面 WebView 的 JavaScript 输出目标，以兼容较旧的 WebView2 Runtime。若启动失败页面提示 WebView2 相关异常，应先更新 Microsoft Edge WebView2 Runtime 后重试。

## 开发

```bash
npm install
npm run desktop
```

## 校验

```bash
npm test
npm run test:rust
```
